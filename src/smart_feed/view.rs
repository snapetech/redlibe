#![allow(clippy::cmp_owned)]

use super::channel::ChannelRule;
use super::cluster;
use super::csrf;
use super::presets::{default_preset, preset, Lens};
use super::rank::{build_score_ctx, now_ts, ScoreResult};
use crate::config::get_setting;
use crate::server::{RequestExt, ResponseExt};
use crate::utils::{info, Preferences, Post};
use askama::Template;
use cookie::Cookie;
use hyper::{Body, Request, Response};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use crate::state::{State, STATE};

#[derive(Debug, Default, Deserialize)]
struct FeedQuery {
	lens: Option<String>,
	preset: Option<String>,
	clusters: Option<String>,
	density: Option<String>,
	limit: Option<u32>,
}

pub struct FeedItem {
	pub post: Post,
	pub is_read: bool,
	pub is_saved: bool,
	pub why: Vec<String>,
	pub cluster_id: Option<String>,
	pub cluster_size: usize,
}

#[derive(Template)]
#[template(path = "feed.html")]
struct FeedTemplate {
	channel_slug: String,
	channel_title: String,
	lens: String,
	preset: String,
	preset_title: String,
	prefs: Preferences,
	url: String,
	items: Vec<FeedItem>,
	csrf: String,
}

fn local_state_enabled() -> bool {
	matches!(get_setting("REDLIB_ENABLE_LOCAL_STATE"), Some(v) if v == "on")
}

fn ensure_sid(req: &Request<Body>, res: &mut Response<Body>) -> Option<String> {
	if !local_state_enabled() {
		return None;
	}
	if let Some(c) = req.cookie("rl_sid") {
		return Some(c.value().to_string());
	}
	let sid = uuid::Uuid::new_v4().to_string();
	let cookie = Cookie::build(("rl_sid", sid.clone()))
		.path("/")
		.http_only(true)
		.same_site(cookie::SameSite::Lax)
		.max_age(cookie::time::Duration::days(365))
		.finish();
	res.insert_cookie(cookie);
	Some(sid)
}

fn build_my_subs_rule(prefs: &Preferences) -> Result<(String, ChannelRule), String> {
	if prefs.subscriptions.is_empty() {
		return Err("Subscribe to some subreddits first (Settings → Subscriptions).".to_string());
	}
	let mut rule = ChannelRule::default();
	rule.sources.subscriptions = true;
	Ok(("My Subs".into(), rule))
}

fn passes_mutes(title: &str, domain: &str, subreddit: &str, mutes: &[crate::state::MuteRule]) -> bool {
	let t = title.to_lowercase();
	let d = domain.to_lowercase();
	let s = subreddit.to_lowercase();
	for m in mutes {
		let p = m.pattern.to_lowercase();
		match m.rule_type.as_str() {
			"keyword" => {
				if t.contains(&p) {
					return false;
				}
			}
			"domain" => {
				if d == p || d.ends_with(&format!(".{p}")) {
					return false;
				}
			}
			"subreddit" => {
				if s == p {
					return false;
				}
			}
			_ => {}
		}
	}
	true
}

pub async fn view(req: Request<Body>) -> Result<Response<Body>, String> {
	let prefs = Preferences::new(&req);
	let url = req.uri().to_string();

	let q: FeedQuery = serde_urlencoded::from_str(req.uri().query().unwrap_or("")).unwrap_or_default();
	let lens = Lens::parse(q.lens.as_deref().unwrap_or("reader"));
	let preset_obj = preset(lens, q.preset.as_deref().unwrap_or(""));

	let channel_slug = req.param("channel").unwrap_or_else(|| "my-subs".to_string());
	let mut res = Response::new(Body::empty());

	// Determine user_key (sid) if local state enabled
	let user_key = ensure_sid(&req, &mut res);

	// System channel: my-subs
	let (channel_title, rule) = if channel_slug == "my-subs" {
		match build_my_subs_rule(&prefs) {
			Ok(v) => v,
			Err(msg) => return info(req, &msg).await,
		}
	} else {
		// For now: if you add channel persistence, load ChannelRule JSON from DB here.
		return info(req, "Only channel 'my-subs' is wired in this skeleton.").await;
	};

	// Fetch posts (use existing Post::fetch + Redlib's JSON caching)
	let limit = q.limit.unwrap_or(100).min(200);
	let subs = prefs.subscriptions.join("+");

	let mut path = format!(
		"/r/{}/{sort}.json?limit={limit}&raw_json=1",
		subs.replace('+', "%2B"),
		sort = preset_obj.upstream_sort
	);
	if let Some(t) = preset_obj.upstream_t {
		path.push_str(&format!("&t={t}"));
	}

	let (posts, _after) = Post::fetch(&path, false).await?;

	// Load local mutes/read-state/saved-state
	let (mutes, read_map, saved_map) = if let (Some(ref key), true) = (user_key.clone(), local_state_enabled()) {
		if let State::Sqlite(store) = &*STATE {
			let mutes = store.list_mutes(key, "global").await.unwrap_or_default();
			let ids: Vec<String> = posts.iter().map(|p| p.id.clone()).collect();
			let read_map = store.get_read_map(key, &ids).await.unwrap_or_default();
			let saved_map = store.get_saved_map(key, &ids).await.unwrap_or_default();
			(mutes, read_map, saved_map)
		} else {
			(Vec::new(), HashMap::new(), HashMap::new())
		}
	} else {
		(Vec::new(), HashMap::new(), HashMap::new())
	};

	// Filter + gate + score
	let now = now_ts();
	let mut scored: Vec<(FeedItem, f64)> = Vec::new();

	for p in posts {
		// apply mutes
		if !passes_mutes(&p.title, &p.domain, &p.community, &mutes) {
			continue;
		}

		// build score ctx
		let domain_is_priority = false;
		let is_pinned_source = false;
		let ctx = build_score_ctx(&p, now, domain_is_priority, is_pinned_source);

		// gate (preset gate + rule gates)
		if !(preset_obj.gate)(&ctx) {
			continue;
		}
		if ctx.num_comments < rule.gates.min_comments {
			continue;
		}
		if ctx.score < rule.gates.min_score {
			continue;
		}
		if rule.gates.max_age_hours > 0 && ctx.age_hours > rule.gates.max_age_hours as f64 {
			continue;
		}

		let ScoreResult { score, mut why } = (preset_obj.score)(&ctx);

		let is_read = read_map.get(&p.id).copied().unwrap_or(false);
		let is_saved = saved_map.get(&p.id).copied().unwrap_or(false);
		if !local_state_enabled() {
			// Don't imply read-state when disabled.
		} else if !is_read {
			why.insert(0, "Unread".into());
		}

		scored.push((
			FeedItem {
				post: p,
				is_read,
				is_saved,
				why,
				cluster_id: None,
				cluster_size: 1,
			},
			score,
		));
	}

	scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

	// Optional clustering (per-page)
	let clusters_on = q.clusters.as_deref().unwrap_or(&rule.presentation.clusters) == "on";
	if clusters_on {
		let title_items: Vec<(String, String)> = scored.iter().map(|(fi, _)| (fi.post.id.clone(), fi.post.title.clone())).collect();
		let clusters = cluster::cluster_titles(&title_items, 4);

		// Build post_id -> (cluster_id, size)
		let mut map: HashMap<String, (String, usize)> = HashMap::new();
		for (cid, c) in &clusters {
			let size = c.members.len();
			if size <= 1 {
				continue;
			}
			for pid in &c.members {
				map.insert(pid.clone(), (cid.clone(), size));
			}
		}

		for (fi, _s) in scored.iter_mut() {
			if let Some((cid, size)) = map.get(&fi.post.id) {
				fi.cluster_id = Some(cid.clone());
				fi.cluster_size = *size;
			}
		}
	}

	let items: Vec<FeedItem> = scored.into_iter().map(|(fi, _)| fi).collect();

	// Batch update "seen" for local state
	if let (Some(ref key), true) = (user_key.clone(), local_state_enabled()) {
		if let State::Sqlite(store) = &*STATE {
			let ids: Vec<String> = items.iter().map(|i| i.post.id.clone()).collect();
			let _ = store.upsert_seen(key, &ids, now).await;
		}
	}

	// Render response — insert csrf cookie if needed
	let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);
	let body = FeedTemplate {
		channel_slug,
		channel_title,
		lens: lens.as_str().into(),
		preset: preset_obj.slug.into(),
		preset_title: preset_obj.title.into(),
		prefs,
		url,
		items,
		csrf: csrf_tok,
	};

	*res.body_mut() = Body::from(body.render().unwrap_or_default());
	*res.status_mut() = hyper::StatusCode::OK;
	res.headers_mut().insert("content-type", hyper::header::HeaderValue::from_static("text/html; charset=utf-8"));
	Ok(res)
}
