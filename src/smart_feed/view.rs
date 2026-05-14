#![allow(clippy::cmp_owned)]

use super::channel::ChannelRule;
use super::cluster;
use super::csrf;
use super::presets::{preset, Lens};
use super::rank::{build_score_ctx, now_ts, ScoreResult};
use super::session::{ensure_sid, local_state_enabled};
use crate::server::RequestExt;
use crate::state::{ChannelRow, PostCacheEntry, State, STATE};
use crate::utils::{info, Post, Preferences};
use askama::Template;
use hyper::{Body, Request, Response};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default, Deserialize)]
struct FeedQuery {
	lens: Option<String>,
	preset: Option<String>,
	clusters: Option<String>,
	limit: Option<u32>,
	filter: Option<String>, // "all" | "unread" | "saved"
	after: Option<String>,
}

pub struct FeedItem {
	pub post: Post,
	pub is_read: bool,
	pub is_saved: bool,
	pub is_archived: bool,
	pub why: Vec<String>,
	pub cluster_id: Option<String>,
	pub cluster_size: usize,
	pub body_preview: String,
}

#[derive(Template)]
#[template(path = "feed.html")]
struct FeedTemplate {
	channel_slug: String,
	channel_title: String,
	channels: Vec<ChannelRow>,
	lens: String,
	preset: String,
	preset_title: String,
	prefs: Preferences,
	url: String,
	items: Vec<FeedItem>,
	csrf: String,
	new_count: usize,
	filter: String,
	total_count: usize,
	unread_count: usize,
	saved_count: usize,
	after: String,
	density: String,
}

/// Build the subreddit join string for a channel rule.
/// Returns Err with a user-facing message if no subs are configured.
pub(super) fn build_fetch_subs(rule: &ChannelRule, prefs: &Preferences) -> Result<String, String> {
	let mut subs: Vec<String> = rule.sources.subreddits.clone();
	if rule.sources.subscriptions {
		for s in &prefs.subscriptions {
			if !subs.iter().any(|x| x.eq_ignore_ascii_case(s)) {
				subs.push(s.clone());
			}
		}
	}
	if subs.is_empty() {
		return Err("No subreddits configured for this channel. Add subreddits in channel settings, or enable 'Include subscriptions'.".to_string());
	}
	Ok(subs.join("+"))
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

fn passes_channel_filters(post: &Post, filters: &super::channel::Filters) -> bool {
	// include_keywords: title must contain at least one (if list is non-empty)
	if !filters.include_keywords.is_empty() {
		let t = post.title.to_lowercase();
		if !filters.include_keywords.iter().any(|k| t.contains(&k.to_lowercase())) {
			return false;
		}
	}
	// exclude_keywords: title must not contain any
	if !filters.exclude_keywords.is_empty() {
		let t = post.title.to_lowercase();
		if filters.exclude_keywords.iter().any(|k| t.contains(&k.to_lowercase())) {
			return false;
		}
	}
	// domains_allow: if set, domain must be in list
	if !filters.domains_allow.is_empty() {
		let d = post.domain.to_lowercase();
		if !filters.domains_allow.iter().any(|dom| {
			let dom = dom.to_lowercase();
			d == dom || d.ends_with(&format!(".{dom}"))
		}) {
			return false;
		}
	}
	// domains_block: domain must not be in list
	if !filters.domains_block.is_empty() {
		let d = post.domain.to_lowercase();
		if filters.domains_block.iter().any(|dom| {
			let dom = dom.to_lowercase();
			d == dom || d.ends_with(&format!(".{dom}"))
		}) {
			return false;
		}
	}
	// media_types: post_type must be in allowed list
	if !filters.media_types.is_empty() && !filters.media_types.iter().any(|t| t == "all") {
		if !filters.media_types.iter().any(|t| t == &post.post_type) {
			return false;
		}
	}
	// nsfw
	if filters.nsfw == "hide" && post.nsfw {
		return false;
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
	let mut res = hyper::Response::new(Body::empty());

	let user_key = ensure_sid(&req, &mut res);

	// Load channel rule
	let (channel_title, rule) = if channel_slug == "my-subs" {
		match build_my_subs_rule(&prefs) {
			Ok(v) => v,
			Err(msg) => return info(req, &msg).await,
		}
	} else {
		// Load custom channel from DB
		let key = match &user_key {
			Some(k) => k.clone(),
			None => return info(req, "Enable local state (REDLIB_ENABLE_LOCAL_STATE=on) to use custom channels.").await,
		};
		if let State::Sqlite(store) = &*STATE {
			match store.get_channel(&key, &channel_slug).await? {
				Some(row) => {
					let rule: ChannelRule = serde_json::from_str(&row.rule_json).map_err(|e| e.to_string())?;
					(row.title, rule)
				}
				None => return info(req, &format!("Channel '{}' not found.", channel_slug)).await,
			}
		} else {
			return info(req, "Enable local state (REDLIB_ENABLE_LOCAL_STATE=on) to use custom channels.").await;
		}
	};

	// Build fetch path from rule sources
	let density = rule.presentation.density.clone();
	let rule_filters = rule.filters.clone();
	let subs = match build_fetch_subs(&rule, &prefs) {
		Ok(s) => s,
		Err(msg) => return info(req, &msg).await,
	};
	let limit = q.limit.unwrap_or(100).min(200);
	let mut path = format!("/r/{}/{sort}.json?limit={limit}&raw_json=1", subs.replace('+', "%2B"), sort = preset_obj.upstream_sort);
	if let Some(t) = preset_obj.upstream_t {
		path.push_str(&format!("&t={t}"));
	}
	if let Some(ref after_tok) = q.after {
		if !after_tok.is_empty() {
			path.push_str(&format!("&after={}", after_tok));
		}
	}

	let (posts, after_cursor) = Post::fetch(&path, false).await?;

	// Cache post metadata (for saved posts view) — fire and forget
	if local_state_enabled() {
		if let State::Sqlite(store) = &*STATE {
			let entries: Vec<PostCacheEntry> = posts
				.iter()
				.map(|p| PostCacheEntry {
					post_id: p.id.clone(),
					title: p.title.clone(),
					community: p.community.clone(),
					domain: p.domain.clone(),
					permalink: p.permalink.clone(),
					score: p.score.1.parse::<i64>().unwrap_or(0),
					comments: p.comments.1.parse::<i64>().unwrap_or(0),
					created_utc: p.created_ts as i64,
				})
				.collect();
			let _ = store.cache_posts(entries).await;
		}
	}

	// Load local state (mutes, read/saved maps, previous visit timestamp)
	let now = now_ts();
	let (mutes, read_map, saved_map, archived_set, prev_visit) = if let (Some(ref key), true) = (user_key.clone(), local_state_enabled()) {
		if let State::Sqlite(store) = &*STATE {
			let mut mutes = store.list_mutes(key, "global").await.unwrap_or_default();
			let channel_scope = format!("channel:{}", channel_slug);
			let mut channel_mutes = store.list_mutes(key, &channel_scope).await.unwrap_or_default();
			mutes.append(&mut channel_mutes);
			let ids: Vec<String> = posts.iter().map(|p| p.id.clone()).collect();
			let read_map = store.get_read_map(key, &ids).await.unwrap_or_default();
			let saved_map = store.get_saved_map(key, &ids).await.unwrap_or_default();
			let archived_set = store.get_archived_set(key, &ids).await.unwrap_or_default();
			let prev_visit = store.get_last_visit(key, &channel_slug).await.unwrap_or(None);
			(mutes, read_map, saved_map, archived_set, prev_visit)
		} else {
			(Vec::new(), HashMap::new(), HashMap::new(), HashSet::new(), None)
		}
	} else {
		(Vec::new(), HashMap::new(), HashMap::new(), HashSet::new(), None)
	};

	// Load channel list for quick-nav
	let channels: Vec<ChannelRow> = if let (Some(ref key), true) = (user_key.clone(), local_state_enabled()) {
		if let State::Sqlite(store) = &*STATE {
			store.list_channels(key).await.unwrap_or_default()
		} else {
			Vec::new()
		}
	} else {
		Vec::new()
	};

	// Filter + gate + score
	let filter = q.filter.as_deref().unwrap_or("all");
	let mut scored: Vec<(FeedItem, f64)> = Vec::new();
	let mut new_count: usize = 0;

	for p in posts {
		// Skip archived posts unless explicitly viewing archived filter
		if archived_set.contains(&p.id) && filter != "archived" {
			continue;
		}

		if !passes_mutes(&p.title, &p.domain, &p.community, &mutes) {
			continue;
		}
		if !passes_channel_filters(&p, &rule_filters) {
			continue;
		}
		let ctx = build_score_ctx(&p, now, false, false);
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
		let is_archived = archived_set.contains(&p.id);

		// Digest: mark posts new since last visit
		let is_new_since_visit = prev_visit.map(|ts| p.created_ts as i64 > ts).unwrap_or(false);
		if local_state_enabled() && is_new_since_visit {
			why.insert(0, "New".into());
			new_count += 1;
		} else if local_state_enabled() && !is_read {
			why.insert(0, "Unread".into());
		}

		let body_preview = if p.post_type == "self" && !p.body.is_empty() {
			crate::utils::plain_text_preview(&p.body, 160)
		} else {
			String::new()
		};
		scored.push((
			FeedItem {
				post: p,
				is_read,
				is_saved,
				is_archived,
				why,
				cluster_id: None,
				cluster_size: 1,
				body_preview,
			},
			score,
		));
	}

	scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

	// Optional clustering
	let clusters_on = q.clusters.as_deref().unwrap_or(&rule.presentation.clusters) == "on";
	if clusters_on {
		let title_items: Vec<(String, String)> = scored.iter().map(|(fi, _)| (fi.post.id.clone(), fi.post.title.clone())).collect();
		let clusters = cluster::cluster_titles(&title_items, 4);

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

		for (fi, _) in scored.iter_mut() {
			if let Some((cid, size)) = map.get(&fi.post.id) {
				fi.cluster_id = Some(cid.clone());
				fi.cluster_size = *size;
			}
		}
	}

	// Apply filter
	let total_count = scored.len();
	let unread_count = scored.iter().filter(|(fi, _)| !fi.is_read).count();
	let saved_count = scored.iter().filter(|(fi, _)| fi.is_saved).count();
	let scored: Vec<(FeedItem, f64)> = match filter {
		"unread" => scored.into_iter().filter(|(fi, _)| !fi.is_read).collect(),
		"saved" => scored.into_iter().filter(|(fi, _)| fi.is_saved).collect(),
		"archived" => scored.into_iter().filter(|(fi, _)| fi.is_archived).collect(),
		_ => scored,
	};

	let items: Vec<FeedItem> = scored.into_iter().map(|(fi, _)| fi).collect();

	// Batch "seen" update + update last visit timestamp
	if let (Some(ref key), true) = (user_key.clone(), local_state_enabled()) {
		if let State::Sqlite(store) = &*STATE {
			let ids: Vec<String> = items.iter().map(|i| i.post.id.clone()).collect();
			let _ = store.upsert_seen(key, &ids, now).await;
			let _ = store.update_last_visit(key, &channel_slug, now).await;
		}
	}

	let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);
	let body = FeedTemplate {
		channel_slug,
		channel_title,
		channels,
		lens: lens.as_str().into(),
		preset: preset_obj.slug.into(),
		preset_title: preset_obj.title.into(),
		prefs,
		url,
		items,
		csrf: csrf_tok,
		new_count,
		filter: filter.to_string(),
		total_count,
		unread_count,
		saved_count,
		after: after_cursor,
		density,
	};

	*res.body_mut() = Body::from(body.render().unwrap_or_default());
	*res.status_mut() = hyper::StatusCode::OK;
	res
		.headers_mut()
		.insert("content-type", hyper::header::HeaderValue::from_static("text/html; charset=utf-8"));
	Ok(res)
}
