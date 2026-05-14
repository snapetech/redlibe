#![allow(clippy::cmp_owned)]

// CRATES
use crate::auth::{update_session_cookie, AuthContext};
use crate::client::json;
use crate::server::RequestExt;
use crate::utils::{
	error, filter_media_only, filter_posts, filter_posts_by_content, filter_read_posts, format_url, get_filter_domains, get_filter_flairs, get_filter_keywords, get_filters,
	get_read_ids, nsfw_landing, param, redirect, setting, template, Post, Preferences, User,
};
use crate::{config, utils};
use askama::Template;
use chrono::DateTime;
use htmlescape::decode_html;
use hyper::{Body, Request, Response};
use time::{macros::format_description, OffsetDateTime};

// STRUCTS
#[derive(Template)]
#[template(path = "user.html")]
struct UserTemplate {
	user: User,
	posts: Vec<Post>,
	sort: (String, String),
	ends: (String, String),
	/// "overview", "comments", or "submitted"
	listing: String,
	prefs: Preferences,
	url: String,
	redirect_url: String,
	/// Whether the user themself is filtered.
	is_filtered: bool,
	/// Whether all fetched posts are filtered (to differentiate between no posts fetched in the first place,
	/// and all fetched posts being filtered).
	all_posts_filtered: bool,
	/// Whether all posts were hidden because they are NSFW (and user has disabled show NSFW)
	all_posts_hidden_nsfw: bool,
	no_posts: bool,
}

const ACCOUNT_LISTINGS: &[&str] = &["saved", "upvoted", "hidden"];

// FUNCTIONS
pub async fn profile(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);
	let mut username = req.param("name").unwrap_or_else(|| "reddit".to_string());
	let resolved_me = username.eq_ignore_ascii_case("me");
	if resolved_me {
		username = auth.username().unwrap_or_default().to_string();
		if username.is_empty() {
			return Ok(redirect("/login"));
		}
		// Redirect to canonical URL so we never send "me" to Reddit
		let listing = req.param("listing").unwrap_or_default();
		let q = req.uri().query().map(|q| format!("?{q}")).unwrap_or_default();
		let to = if listing.is_empty() {
			format!("/user/{username}{q}")
		} else {
			format!("/user/{username}/{listing}{q}")
		};
		return Ok(redirect(&to));
	}

	let listing = req.param("listing").unwrap_or_else(|| "overview".to_string());
	let path = format!("/user/{}/{listing}.json?{}&raw_json=1", username, req.uri().query().unwrap_or_default(),);
	let url = String::from(req.uri().path_and_query().map_or("", |val| val.as_str()));
	let redirect_url = url[1..].replace('?', "%3F").replace('&', "%26");
	let sort = param(&path, "sort").unwrap_or_default();

	let user = user(&username).await.unwrap_or_default();
	let req_url = req.uri().to_string();
	if user.nsfw && crate::utils::should_be_nsfw_gated(&req, &req_url) {
		return Ok(nsfw_landing(req, req_url).await.unwrap_or_default());
	}

	let filters = get_filters(&req);
	if filters.contains(&["u_", &username].concat()) {
		return Ok(template(&UserTemplate {
			user,
			posts: Vec::new(),
			sort: (sort.clone(), param(&path, "t").unwrap_or_default()),
			ends: (param(&path, "after").unwrap_or_default(), String::new()),
			listing: listing.clone(),
			prefs: Preferences::new(&req),
			url,
			redirect_url,
			is_filtered: true,
			all_posts_filtered: false,
			all_posts_hidden_nsfw: false,
			no_posts: false,
		}));
	}

	let use_authed = ACCOUNT_LISTINGS.contains(&listing.as_str());
	if use_authed && !auth.is_authenticated() {
		return Ok(redirect("/login"));
	}

	let (mut posts, after) = if use_authed {
		let ((mut p, a), session_updated) = Post::fetch_authed(&path, false, &auth).await.map_err(|msg| msg.to_string())?;
		let (_, all_posts_filtered) = filter_posts(&mut p, &filters);
		filter_posts_by_content(&mut p, &get_filter_keywords(&req), &get_filter_flairs(&req), &get_filter_domains(&req));
		if setting(&req, "hide_read") == "on" {
			let read_ids = get_read_ids(&req);
			filter_read_posts(&mut p, &read_ids);
		}
		if setting(&req, "show_only_media") == "on" {
			filter_media_only(&mut p);
		}
		let no_posts = p.is_empty();
		let all_posts_hidden_nsfw = !no_posts && (p.iter().all(|x| x.flags.nsfw) && setting(&req, "show_nsfw") != "on");
		let mut res = template(&UserTemplate {
			user,
			posts: p,
			sort: (sort, param(&path, "t").unwrap_or_default()),
			ends: (param(&path, "after").unwrap_or_default(), a),
			listing,
			prefs: Preferences::new(&req),
			url,
			redirect_url,
			is_filtered: false,
			all_posts_filtered,
			all_posts_hidden_nsfw,
			no_posts,
		});
		if let Some(s) = session_updated {
			update_session_cookie(&mut res, &s);
		}
		return Ok(res);
	} else {
		match Post::fetch(&path, false).await {
			Ok((p, a)) => (p, a),
			Err(msg) => return error(req, &msg).await,
		}
	};

	let (_, all_posts_filtered) = filter_posts(&mut posts, &filters);
	filter_posts_by_content(&mut posts, &get_filter_keywords(&req), &get_filter_flairs(&req), &get_filter_domains(&req));
	if setting(&req, "hide_read") == "on" {
		let read_ids = get_read_ids(&req);
		filter_read_posts(&mut posts, &read_ids);
	}
	if setting(&req, "show_only_media") == "on" {
		filter_media_only(&mut posts);
	}
	let no_posts = posts.is_empty();
	let all_posts_hidden_nsfw = !no_posts && (posts.iter().all(|p| p.flags.nsfw) && setting(&req, "show_nsfw") != "on");
	Ok(template(&UserTemplate {
		user,
		posts,
		sort: (sort, param(&path, "t").unwrap_or_default()),
		ends: (param(&path, "after").unwrap_or_default(), after),
		listing,
		prefs: Preferences::new(&req),
		url,
		redirect_url,
		is_filtered: false,
		all_posts_filtered,
		all_posts_hidden_nsfw,
		no_posts,
	}))
}

// USER
async fn user(name: &str) -> Result<User, String> {
	// Build the Reddit JSON API path
	let path: String = format!("/user/{name}/about.json?raw_json=1");

	// Send a request to the url
	json(path, false).await.map(|res| {
		// Grab creation date as unix timestamp
		let created_unix = res["data"]["created"].as_f64().unwrap_or(0.0).round() as i64;
		let created = OffsetDateTime::from_unix_timestamp(created_unix).unwrap_or(OffsetDateTime::UNIX_EPOCH);

		// Closure used to parse JSON from Reddit APIs
		let about = |item| res["data"]["subreddit"][item].as_str().unwrap_or_default().to_string();

		// Parse the JSON output into a User struct
		User {
			name: res["data"]["name"].as_str().unwrap_or(name).to_owned(),
			title: about("title"),
			icon: format_url(&about("icon_img")),
			karma: res["data"]["total_karma"].as_i64().unwrap_or(0),
			created: created.format(format_description!("[month repr:short] [day] '[year repr:last_two]")).unwrap_or_default(),
			banner: about("banner_img"),
			description: about("public_description"),
			nsfw: res["data"]["subreddit"]["over_18"].as_bool().unwrap_or_default(),
		}
	})
}

pub async fn rss(req: Request<Body>) -> Result<Response<Body>, String> {
	if config::get_setting("REDLIB_ENABLE_RSS").is_none() {
		return Ok(error(req, "RSS is disabled on this instance.").await.unwrap_or_default());
	}
	use crate::utils::rewrite_urls;
	use hyper::header::CONTENT_TYPE;
	use rss::{ChannelBuilder, Item};

	// Get user
	let user_str = req.param("name").unwrap_or_default();

	let listing = req.param("listing").unwrap_or_else(|| "overview".to_string());

	// Get path
	let path = format!("/user/{user_str}/{listing}.json?{}&raw_json=1", req.uri().query().unwrap_or_default(),);

	// Get user
	let user_obj = user(&user_str).await.unwrap_or_default();

	// Get posts
	let (posts, _) = Post::fetch(&path, false).await?;

	// Build the RSS feed
	let channel = ChannelBuilder::default()
		.title(user_str)
		.description(user_obj.description)
		.items(
			posts
				.into_iter()
				.map(|post| Item {
					title: Some(post.title.to_string()),
					link: Some(format_url(&utils::get_post_url(&post))),
					author: Some(post.author.name),
					pub_date: Some(DateTime::from_timestamp(post.created_ts as i64, 0).unwrap_or_default().to_rfc2822()),
					content: Some(rewrite_urls(&decode_html(&post.body).unwrap_or_else(|_| post.body.clone()))),
					..Default::default()
				})
				.collect::<Vec<_>>(),
		)
		.build();

	// Serialize the feed to RSS
	let body = channel.to_string().into_bytes();

	// Create the HTTP response
	let mut res = Response::new(Body::from(body));
	res.headers_mut().insert(CONTENT_TYPE, hyper::header::HeaderValue::from_static("application/rss+xml"));

	Ok(res)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_fetching_user() {
	let user = user("spez").await;
	assert!(user.is_ok());
	assert!(user.unwrap().karma > 100);
}
