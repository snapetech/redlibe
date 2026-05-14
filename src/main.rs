// Global specifiers
#![forbid(unsafe_code)]
#![allow(clippy::cmp_owned)]

use cached::proc_macro::cached;
use clap::{Arg, ArgAction, Command};
use std::str::FromStr;
use std::sync::LazyLock;

use futures_lite::FutureExt;
use hyper::Uri;
use hyper::{header::HeaderValue, Body, Request, Response};
use log::{info, warn};
use redlib::client::{canonical_path, proxy, rate_limit_check, upstream_diagnostics_snapshot, upstream_metrics_snapshot_json, upstream_prometheus_metrics, CLIENT};
use redlib::server::{self, RequestExt};
use redlib::utils::{error, redirect, ThemeAssets};
use redlib::{api, auth, comment, config, duplicates, edit, feeds, go, headers, inbox, instance_info, post, search, settings, smart_feed, submit, subreddit, user, vote};

use redlib::client::OAUTH_CLIENT;

// Create Services

// Required for the manifest to be valid
async fn pwa_logo() -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "image/png")
			.body(include_bytes!("../static/logo.png").as_ref().into())
			.unwrap_or_default(),
	)
}

// Required for iOS App Icons
async fn iphone_logo() -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "image/png")
			.body(include_bytes!("../static/apple-touch-icon.png").as_ref().into())
			.unwrap_or_default(),
	)
}

async fn favicon() -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "image/vnd.microsoft.icon")
			.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
			.body(include_bytes!("../static/favicon.ico").as_ref().into())
			.unwrap_or_default(),
	)
}

async fn font() -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "font/woff2")
			.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
			.body(include_bytes!("../static/Inter.var.woff2").as_ref().into())
			.unwrap_or_default(),
	)
}

async fn opensearch() -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "application/opensearchdescription+xml")
			.header("Cache-Control", "public, max-age=1209600, s-maxage=86400")
			.body(include_bytes!("../static/opensearch.xml").as_ref().into())
			.unwrap_or_default(),
	)
}

async fn resource(body: &str, content_type: &str, cache: bool) -> Result<Response<Body>, String> {
	let mut res = Response::builder()
		.status(200)
		.header("content-type", content_type)
		.body(body.to_string().into())
		.unwrap_or_default();

	if cache {
		if let Ok(val) = HeaderValue::from_str("public, max-age=1209600, s-maxage=86400") {
			res.headers_mut().insert("Cache-Control", val);
		}
	}

	Ok(res)
}

fn html_escape(s: &str) -> String {
	s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

async fn prometheus_metrics() -> Result<Response<Body>, String> {
	let mut body = upstream_prometheus_metrics();
	body.push_str(&subreddit::render_cache_prometheus_metrics());
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "text/plain; version=0.0.4; charset=utf-8")
			.header("Cache-Control", "no-store")
			.body(body.into())
			.unwrap_or_default(),
	)
}

async fn upstream_diagnostics_page() -> Result<Response<Body>, String> {
	let upstream_json = upstream_metrics_snapshot_json();
	let upstream_diag = upstream_diagnostics_snapshot();
	let (hits, misses, stores, entries) = subreddit::render_cache_metrics_snapshot();
	let total = hits + misses;
	let hit_ratio = if total == 0 { 0.0 } else { (hits as f64 / total as f64) * 100.0 };
	let html = format!(
		r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Redlibe Diagnostics</title>
<style>
body{{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;background:#161412;color:#efe7dd;margin:0;padding:1.25rem;}}
.wrap{{max-width:960px;margin:0 auto;display:grid;gap:1rem;}}
.card{{background:#211d19;border:1px solid #3a322b;border-radius:12px;padding:1rem;}}
h1,h2{{margin:0 0 .6rem 0;}}
pre{{white-space:pre-wrap;word-break:break-word;background:#130f0c;border-radius:8px;padding:.8rem;border:1px solid #302821;}}
a{{color:#f3bf7a}}
.row{{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:.75rem}}
.stat{{background:#191511;border:1px solid #2f2923;border-radius:10px;padding:.75rem}}
.label{{opacity:.8;font-size:.85rem}}
.value{{font-size:1.05rem;margin-top:.15rem}}
</style>
</head>
<body>
<main class="wrap">
<h1>Redlibe Upstream Diagnostics</h1>
<div class="row">
<div class="stat"><div class="label">Render Cache Hit Ratio</div><div class="value">{:.1}%</div></div>
<div class="stat"><div class="label">Render Cache Hits/Misses</div><div class="value">{}/{} </div></div>
<div class="stat"><div class="label">Render Cache Entries</div><div class="value">{}</div></div>
<div class="stat"><div class="label">Render Cache Stores</div><div class="value">{}</div></div>
</div>
<div class="card">
<h2>OAuth / Token Refresh Status</h2>
<pre>{}</pre>
</div>
<div class="card">
<h2>Upstream Counters (JSON)</h2>
<pre>{}</pre>
</div>
<div class="card">
<h2>Links</h2>
<p><a href="/upstream-metrics.json">/upstream-metrics.json</a> · <a href="/metrics">/metrics</a> · <a href="/settings">/settings</a></p>
</div>
</main>
</body>
</html>"#,
		hit_ratio,
		hits,
		misses,
		entries,
		stores,
		html_escape(&upstream_diag),
		html_escape(&upstream_json)
	);
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "text/html; charset=utf-8")
			.header("Cache-Control", "no-store")
			.body(html.into())
			.unwrap_or_default(),
	)
}

async fn style() -> Result<Response<Body>, String> {
	let mut res = include_str!("../static/style.css").to_string();
	for file in ThemeAssets::iter() {
		res.push('\n');
		let theme = ThemeAssets::get(file.as_ref()).unwrap();
		res.push_str(std::str::from_utf8(theme.data.as_ref()).unwrap());
	}
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "text/css")
			.header("Cache-Control", "no-cache, max-age=0, must-revalidate")
			.body(res.to_string().into())
			.unwrap_or_default(),
	)
}

#[tokio::main]
async fn main() {
	// Load environment variables
	_ = dotenvy::dotenv();

	// Initialize logger
	pretty_env_logger::init();

	let matches = Command::new("Redlib")
		.version(env!("CARGO_PKG_VERSION"))
		.about("Private front-end for Reddit written in Rust ")
		.arg(Arg::new("ipv4-only").short('4').long("ipv4-only").help("Listen on IPv4 only").num_args(0))
		.arg(Arg::new("ipv6-only").short('6').long("ipv6-only").help("Listen on IPv6 only").num_args(0))
		.arg(
			Arg::new("redirect-https")
				.short('r')
				.long("redirect-https")
				.help("Redirect all HTTP requests to HTTPS (no longer functional)")
				.num_args(0),
		)
		.arg(
			Arg::new("address")
				.short('a')
				.long("address")
				.value_name("ADDRESS")
				.help("Sets address to listen on")
				.default_value("[::]")
				.num_args(1),
		)
		.arg(
			Arg::new("port")
				.short('p')
				.long("port")
				.value_name("PORT")
				.env("PORT")
				.help("Port to listen on")
				.default_value("8080")
				.action(ArgAction::Set)
				.num_args(1),
		)
		.arg(
			Arg::new("hsts")
				.short('H')
				.long("hsts")
				.value_name("EXPIRE_TIME")
				.help("HSTS header max-age in seconds (recommended: 15768000 = 6 months)")
				.default_value("15768000")
				.num_args(1),
		)
		.get_matches();

	match rate_limit_check().await {
		Ok(()) => {
			info!("[✅] Rate limit check passed");
		}
		Err(e) => {
			let mut message = format!("Rate limit check failed: {e}");
			message += "\nThis may cause issues with the rate limit.";
			message += "\nPlease report this error with the above information.";
			message += "\nhttps://github.com/redlib-org/redlib/issues/new?assignees=sigaloid&labels=bug&title=%F0%9F%90%9B+Bug+Report%3A+Rate+limit+mismatch";
			warn!("{}", message);
			eprintln!("{message}");
		}
	}

	let address = matches.get_one::<String>("address").unwrap();
	let port = matches.get_one::<String>("port").unwrap();
	let hsts = matches.get_one("hsts").map(|m: &String| m.as_str());

	let ipv4_only = std::env::var("IPV4_ONLY").is_ok() || matches.get_flag("ipv4-only");
	let ipv6_only = std::env::var("IPV6_ONLY").is_ok() || matches.get_flag("ipv6-only");

	let listener = if ipv4_only {
		format!("0.0.0.0:{port}")
	} else if ipv6_only {
		format!("[::]:{port}")
	} else {
		[address, ":", port].concat()
	};

	println!("Starting Redlib...");

	// Begin constructing a server
	let mut app = server::Server::new();

	// Force evaluation of statics. In instance_info case, we need to evaluate
	// the timestamp so deploy date is accurate - in config case, we need to
	// evaluate the configuration to avoid paying penalty at first request -
	// in OAUTH case, optionally retrieve the token at startup to avoid paying
	// the penalty at first request. Keep this lazy by default so SSH-session-only
	// deployments can start even when Reddit's anonymous OAuth flow is unhealthy.

	info!("Evaluating config.");
	LazyLock::force(&config::CONFIG);
	info!("Evaluating instance info.");
	LazyLock::force(&instance_info::INSTANCE_INFO);
	if std::env::var("REDLIB_EAGER_OAUTH").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true")) {
		info!("Creating OAUTH client.");
		LazyLock::force(&OAUTH_CLIENT);
	}
	info!("Initializing local state store.");
	LazyLock::force(&redlib::state::STATE);

	// Define default headers (added to all responses)
	app.default_headers = headers! {
		"Referrer-Policy" => "no-referrer",
		"X-Content-Type-Options" => "nosniff",
		"X-Frame-Options" => "DENY",
		"X-XSS-Protection" => "0",
		"Permissions-Policy" => "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()",
		"Content-Security-Policy" => "default-src 'none'; font-src 'self'; script-src 'self' blob:; manifest-src 'self'; media-src 'self' data: blob: about:; style-src 'self' 'unsafe-inline'; base-uri 'none'; img-src 'self' data:; form-action 'self'; frame-ancestors 'none'; connect-src 'self'; worker-src blob:;"
	};

	if let Some(expire_time) = hsts {
		if let Ok(val) = HeaderValue::from_str(&format!("max-age={expire_time}; includeSubdomains")) {
			app.default_headers.insert("Strict-Transport-Security", val);
		}
	}

	// Read static files
	app.at("/style.css").get(|_| style().boxed());
	app
		.at("/manifest.json")
		.get(|_| resource(include_str!("../static/manifest.json"), "application/json", false).boxed());
	app.at("/robots.txt").get(|_| {
		resource(
			if match config::get_setting("REDLIB_ROBOTS_DISABLE_INDEXING") {
				Some(val) => val == "on",
				None => false,
			} {
				"User-agent: *\nDisallow: /"
			} else {
				"User-agent: *\nDisallow: /u/\nDisallow: /user/"
			},
			"text/plain",
			true,
		)
		.boxed()
	});
	app.at("/sw.js").get(|_| resource(include_str!("../static/sw.js"), "application/javascript", false).boxed());
	app.at("/favicon.ico").get(|_| favicon().boxed());
	app.at("/logo.png").get(|_| pwa_logo().boxed());
	app.at("/Inter.var.woff2").get(|_| font().boxed());
	app.at("/touch-icon-iphone.png").get(|_| iphone_logo().boxed());
	app.at("/apple-touch-icon.png").get(|_| iphone_logo().boxed());
	app.at("/opensearch.xml").get(|_| opensearch().boxed());
	app
		.at("/playHLSVideo.js")
		.get(|_| resource(include_str!("../static/playHLSVideo.js"), "text/javascript", false).boxed());
	app
		.at("/hls.min.js")
		.get(|_| resource(include_str!("../static/hls.min.js"), "text/javascript", false).boxed());
	app
		.at("/highlighted.js")
		.get(|_| resource(include_str!("../static/highlighted.js"), "text/javascript", false).boxed());
	app
		.at("/check_update.js")
		.get(|_| resource(include_str!("../static/check_update.js"), "text/javascript", false).boxed());
	app.at("/copy.js").get(|_| resource(include_str!("../static/copy.js"), "text/javascript", false).boxed());
	app
		.at("/settings.js")
		.get(|_| resource(include_str!("../static/settings.js"), "text/javascript", false).boxed());
	app.at("/power.js").get(|_| resource(include_str!("../static/power.js"), "text/javascript", false).boxed());
	app.at("/upstream-metrics.json").get(|_| {
		async {
			Ok::<Response<Body>, String>(
				Response::builder()
					.status(200)
					.header("content-type", "application/json")
					.header("Cache-Control", "no-store")
					.body(upstream_metrics_snapshot_json().into())
					.unwrap_or_default(),
			)
		}
		.boxed()
	});
	app.at("/metrics").get(|_| prometheus_metrics().boxed());
	app.at("/diagnostics/upstream").get(|_| upstream_diagnostics_page().boxed());

	app.at("/commits.atom").get(|_| async move { proxy_commit_info().await }.boxed());
	app.at("/instances.json").get(|_| async move { proxy_instances().await }.boxed());

	// Proxy media through Redlib
	app.at("/vid/:id/:size").get(|r| proxy(r, "https://v.redd.it/{id}/DASH_{size}").boxed());
	app.at("/hls/:id/*path").get(|r| proxy(r, "https://v.redd.it/{id}/{path}").boxed());
	app.at("/img/*path").get(|r| proxy(r, "https://i.redd.it/{path}").boxed());
	app.at("/thumb/:point/:id").get(|r| proxy(r, "https://{point}.thumbs.redditmedia.com/{id}").boxed());
	app.at("/emoji/:id/:name").get(|r| proxy(r, "https://emoji.redditmedia.com/{id}/{name}").boxed());
	app
		.at("/emote/:subreddit_id/:filename")
		.get(|r| proxy(r, "https://reddit-econ-prod-assets-permanent.s3.amazonaws.com/asset-manager/{subreddit_id}/{filename}").boxed());
	app
		.at("/preview/:loc/award_images/:fullname/:id")
		.get(|r| proxy(r, "https://{loc}view.redd.it/award_images/{fullname}/{id}").boxed());
	app.at("/preview/:loc/:id").get(|r| proxy(r, "https://{loc}view.redd.it/{id}").boxed());
	app.at("/style/*path").get(|r| proxy(r, "https://styles.redditmedia.com/{path}").boxed());
	app.at("/static/*path").get(|r| proxy(r, "https://www.redditstatic.com/{path}").boxed());

	// Browse user profile
	app
		.at("/u/:name")
		.get(|r| async move { Ok(redirect(&format!("/user/{}", r.param("name").unwrap_or_default()))) }.boxed());
	app.at("/u/:name/comments/:id/:title").get(|r| post::item(r).boxed());
	app.at("/u/:name/comments/:id/:title/:comment_id").get(|r| post::item(r).boxed());

	app.at("/user/[deleted]").get(|req| error(req, "User has deleted their account").boxed());
	app.at("/user/:name.rss").get(|r| user::rss(r).boxed());
	app.at("/user/:name").get(|r| user::profile(r).boxed());
	app.at("/user/:name/:listing").get(|r| user::profile(r).boxed());
	app.at("/user/:name/comments/:id").get(|r| post::item(r).boxed());
	app.at("/user/:name/comments/:id/:title").get(|r| post::item(r).boxed());
	app.at("/user/:name/comments/:id/:title/:comment_id").get(|r| post::item(r).boxed());

	// Configure settings
	app.at("/settings").get(|r| settings::get(r).boxed()).post(|r| settings::set(r).boxed());
	app.at("/settings/restore").get(|r| settings::restore(r).boxed());
	app.at("/settings/encoded-restore").post(|r| settings::encoded_restore(r).boxed());
	app.at("/settings/export-json").get(|r| settings::export_json(r).boxed());
	app.at("/settings/export-env").get(|r| settings::export_env(r).boxed());
	app.at("/settings/import-json").post(|r| settings::import_json(r).boxed());
	app.at("/settings/update").get(|r| settings::update(r).boxed());

	// --- redlib-extended: Auth, voting, REST API ---

	// Auth: login page, Reddit OAuth flow, SSH browser import, logout
	app.at("/login").get(|r| auth::login_page(r).boxed());
	app
		.at("/login/reddit")
		.get(|_| async { Ok(redirect("/login")) }.boxed())
		.post(|r| auth::login_reddit(r).boxed());
	app
		.at("/login/ssh-import")
		.get(|_| async { Ok(redirect("/login")) }.boxed())
		.post(|r| auth::login_ssh_import(r).boxed());
	app
		.at("/login/local-import")
		.get(|_| async { Ok(redirect("/login")) }.boxed())
		.post(|r| auth::login_local_import(r).boxed());
	app.at("/auth/callback").get(|r| auth::oauth_callback(r).boxed());
	app.at("/logout").post(|r| auth::logout(r).boxed());
	app.at("/auth/switch").post(|r| auth::switch_account(r).boxed());
	app.at("/auth/remove").post(|r| auth::remove_account(r).boxed());

	// Voting, save/unsave, comment reply
	app.at("/vote").post(|r| vote::submit(r).boxed());
	app.at("/save").post(|r| vote::save(r).boxed());
	app.at("/comment").post(|r| comment::submit(r).boxed());
	app.at("/edit").post(|r| edit::submit(r).boxed());
	app.at("/search-save").post(|r| search::save(r).boxed());
	app.at("/search-unsave").post(|r| search::unsave(r).boxed());

	// Inbox and private messages
	app.at("/inbox").get(|r| inbox::list(r).boxed());
	app.at("/inbox/compose").get(|r| inbox::compose_get(r).boxed()).post(|r| inbox::compose_post(r).boxed());
	app.at("/inbox/read").post(|r| inbox::read_message(r).boxed());
	app.at("/inbox/read-all").post(|r| inbox::read_all(r).boxed());

	// Custom feeds (internal named multireddits)
	app.at("/feeds").get(|r| feeds::get(r).boxed()).post(|r| feeds::post(r).boxed());
	app.at("/feed/:name").get(|r| feeds::redirect_to_feed(r).boxed());

	// Smart feed engine: ranked/filtered feed with Lenses + Presets
	app.at("/reader/saved").get(|r| smart_feed::saved_view(r).boxed());
	app.at("/reader/:channel").get(|r| smart_feed::view(r).boxed());
	app.at("/action/mark_read").post(|r| smart_feed::action_mark_read(r).boxed());
	app.at("/action/mark_unread").post(|r| smart_feed::action_mark_unread(r).boxed());
	app.at("/action/save").post(|r| smart_feed::action_save(r).boxed());
	app.at("/action/unsave").post(|r| smart_feed::action_unsave(r).boxed());
	app.at("/action/mute_keyword").post(|r| smart_feed::action_mute_keyword(r).boxed());
	app.at("/action/mute_domain").post(|r| smart_feed::action_mute_domain(r).boxed());
	app.at("/action/mute_subreddit").post(|r| smart_feed::action_mute_subreddit(r).boxed());
	app.at("/action/unmute").post(|r| smart_feed::action_unmute(r).boxed());
	app.at("/action/mark_all_read").post(|r| smart_feed::action_mark_all_read(r).boxed());
	app.at("/action/archive").post(|r| smart_feed::action_archive(r).boxed());
	app.at("/action/unarchive").post(|r| smart_feed::action_unarchive(r).boxed());
	app.at("/action/open").get(|r| smart_feed::action_open(r).boxed());
	app.at("/action/sync_subscriptions").post(|r| crate::auth::sync_subscriptions(r).boxed());

	// Channel management + cluster view + mutes + saved
	app
		.at("/channels")
		.get(|r| smart_feed::channels_list(r).boxed())
		.post(|r| smart_feed::channels_create(r).boxed());
	app
		.at("/channels/:slug")
		.get(|r| smart_feed::channels_edit(r).boxed())
		.post(|r| smart_feed::channels_update(r).boxed());
	app.at("/channels/:slug/delete").post(|r| smart_feed::channels_delete(r).boxed());
	app.at("/channels/:slug/move").post(|r| smart_feed::channels_move(r).boxed());
	app.at("/cluster/:id").get(|r| smart_feed::cluster_view(r).boxed());
	app.at("/mutes").get(|r| smart_feed::mutes_list(r).boxed());
	app.at("/mutes/export.json").get(|r| smart_feed::action_export_mutes(r).boxed());
	app.at("/mutes/import").post(|r| smart_feed::action_import_mutes(r).boxed());
	app.at("/stats").get(|r| smart_feed::stats_view(r).boxed());
	app.at("/api/reader/unread_count").get(|r| smart_feed::api_unread_count(r).boxed());

	// REST API
	app.at("/api/v1/r/:sub").get(|r| api::subreddit_listing(r).boxed());
	app.at("/api/v1/r/:sub/comments/:id").get(|r| api::post_comments(r).boxed());
	app.at("/api/v1/me").get(|r| api::me(r).boxed());
	app.at("/api/v1/vote").post(|r| api::api_vote(r).boxed());

	// RSS Subscriptions
	app.at("/r/:sub.rss").get(|r| subreddit::rss(r).boxed());

	// Subreddit services
	app
		.at("/r/:sub")
		.get(|r| subreddit::community(r).boxed())
		.post(|r| subreddit::add_quarantine_exception(r).boxed());

	app
		.at("/r/u_:name")
		.get(|r| async move { Ok(redirect(&format!("/user/{}", r.param("name").unwrap_or_default()))) }.boxed());

	app.at("/r/:sub/subscribe").post(|r| subreddit::subscriptions_filters(r).boxed());
	app.at("/r/:sub/unsubscribe").post(|r| subreddit::subscriptions_filters(r).boxed());
	app.at("/r/:sub/filter").post(|r| subreddit::subscriptions_filters(r).boxed());
	app.at("/r/:sub/unfilter").post(|r| subreddit::subscriptions_filters(r).boxed());

	app.at("/mark-read").post(|r| subreddit::mark_read(r).boxed());
	app.at("/comment-collapse").post(|r| post::comment_collapse(r).boxed());

	app.at("/r/:sub/submit").get(|r| submit::get(r).boxed()).post(|r| submit::post(r).boxed());

	app.at("/r/:sub/comments/:id").get(|r| post::item(r).boxed());
	app.at("/r/:sub/comments/:id/:title").get(|r| post::item(r).boxed());
	app.at("/r/:sub/comments/:id/:title/:comment_id").get(|r| post::item(r).boxed());
	app.at("/comments/:id").get(|r| post::item(r).boxed());
	app.at("/comments/:id/comments").get(|r| post::item(r).boxed());
	app.at("/comments/:id/comments/:comment_id").get(|r| post::item(r).boxed());
	app.at("/comments/:id/:title").get(|r| post::item(r).boxed());
	app.at("/comments/:id/:title/:comment_id").get(|r| post::item(r).boxed());

	app.at("/r/:sub/duplicates/:id").get(|r| duplicates::item(r).boxed());
	app.at("/r/:sub/duplicates/:id/:title").get(|r| duplicates::item(r).boxed());
	app.at("/duplicates/:id").get(|r| duplicates::item(r).boxed());
	app.at("/duplicates/:id/:title").get(|r| duplicates::item(r).boxed());

	app.at("/r/:sub/search").get(|r| search::find(r).boxed());

	app
		.at("/r/:sub/w")
		.get(|r| async move { Ok(redirect(&format!("/r/{}/wiki", r.param("sub").unwrap_or_default()))) }.boxed());
	app
		.at("/r/:sub/w/*page")
		.get(|r| async move { Ok(redirect(&format!("/r/{}/wiki/{}", r.param("sub").unwrap_or_default(), r.param("wiki").unwrap_or_default()))) }.boxed());
	app.at("/r/:sub/wiki").get(|r| subreddit::wiki(r).boxed());
	app.at("/r/:sub/wiki/*page").get(|r| subreddit::wiki(r).boxed());

	app.at("/r/:sub/about/sidebar").get(|r| subreddit::sidebar(r).boxed());

	app.at("/r/:sub/:sort").get(|r| subreddit::community(r).boxed());

	// Front page
	app.at("/").get(|r| subreddit::community(r).boxed());

	// View Reddit wiki
	app.at("/w").get(|_| async { Ok(redirect("/wiki")) }.boxed());
	app
		.at("/w/*page")
		.get(|r| async move { Ok(redirect(&format!("/wiki/{}", r.param("page").unwrap_or_default()))) }.boxed());
	app.at("/wiki").get(|r| subreddit::wiki(r).boxed());
	app.at("/wiki/*page").get(|r| subreddit::wiki(r).boxed());

	// Search all of Reddit
	app.at("/search").get(|r| search::find(r).boxed());

	// Quick jump to subreddit
	app.at("/go").get(|r| go::get_go(r).boxed());

	// Handle about pages
	app.at("/about").get(|req| error(req, "About pages aren't added yet").boxed());

	// Instance info page
	app.at("/info").get(|r| instance_info::instance_info(r).boxed());
	app.at("/info.:extension").get(|r| instance_info::instance_info(r).boxed());

	// Handle obfuscated share links.
	// Note that this still forces the server to follow the share link to get to the post, so maybe this wants to be updated with a warning before it follow it
	app.at("/r/:sub/s/:id").get(|req: Request<Body>| {
		Box::pin(async move {
			let sub = req.param("sub").unwrap_or_default();
			match req.param("id").as_deref() {
				// Share link
				Some(id) if (8..12).contains(&id.len()) => match canonical_path(format!("/r/{sub}/s/{id}"), 3).await {
					Ok(Some(path)) => Ok(redirect(&path)),
					Ok(None) => error(req, "Post ID is invalid. It may point to a post on a community that has been banned.").await,
					Err(e) => error(req, &e).await,
				},

				// Error message for unknown pages
				_ => error(req, "Nothing here").await,
			}
		})
	});

	app.at("/:id").get(|req: Request<Body>| {
		Box::pin(async move {
			match req.param("id").as_deref() {
				// Sort front page
				Some("best" | "hot" | "new" | "top" | "rising" | "controversial") => subreddit::community(req).await,

				// Short link for post
				Some(id) if (5..8).contains(&id.len()) => match canonical_path(format!("/comments/{id}"), 3).await {
					Ok(path_opt) => match path_opt {
						Some(path) => Ok(redirect(&path)),
						None => error(req, "Post ID is invalid. It may point to a post on a community that has been banned.").await,
					},
					Err(e) => error(req, &e).await,
				},

				// Error message for unknown pages
				_ => error(req, "Nothing here").await,
			}
		})
	});

	// Default service in case no routes match
	app.at("/*").get(|req| error(req, "Nothing here").boxed());

	println!("Running Redlib v{} on {listener}!", env!("CARGO_PKG_VERSION"));

	let server = app.listen(&listener);

	// Run this server for... forever!
	if let Err(e) = server.await {
		eprintln!("Server error: {e}");
	}
}

pub async fn proxy_commit_info() -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "application/atom+xml")
			.body(Body::from(fetch_commit_info().await))
			.unwrap_or_default(),
	)
}

#[cached(time = 600)]
async fn fetch_commit_info() -> String {
	let uri = Uri::from_str("https://github.com/redlib-org/redlib/commits/main.atom").expect("Invalid URI");

	let resp: Body = CLIENT.get(uri).await.expect("Failed to request GitHub").into_body();

	hyper::body::to_bytes(resp).await.expect("Failed to read body").iter().copied().map(|x| x as char).collect()
}

pub async fn proxy_instances() -> Result<Response<Body>, String> {
	Ok(
		Response::builder()
			.status(200)
			.header("content-type", "application/json")
			.body(Body::from(fetch_instances().await))
			.unwrap_or_default(),
	)
}

#[cached(time = 600)]
async fn fetch_instances() -> String {
	let uri = Uri::from_str("https://raw.githubusercontent.com/redlib-org/redlib-instances/refs/heads/main/instances.json").expect("Invalid URI");

	let resp: Body = CLIENT.get(uri).await.expect("Failed to request GitHub").into_body();

	hyper::body::to_bytes(resp).await.expect("Failed to read body").iter().copied().map(|x| x as char).collect()
}
