use super::channel::{ChannelRule, Filters, Gates, Presentation, Sources};
use super::csrf;
use super::session::{local_state_enabled, require_user_key};
use crate::server::RequestExt;
use crate::state::{ChannelRow, State, STATE};
use crate::utils::{error, info, redirect, Preferences};
use askama::Template;
use hyper::{Body, Request, Response};
use std::collections::HashMap;

// ---- Templates ----

#[derive(Template)]
#[template(path = "channels.html")]
struct ChannelsTemplate {
	prefs: Preferences,
	channels: Vec<ChannelRow>,
	error: String,
	url: String,
	csrf: String,
}

#[derive(Template)]
#[template(path = "channel_edit.html")]
struct ChannelEditTemplate {
	prefs: Preferences,
	slug: String,
	title: String,
	rule: ChannelRule,
	error: String,
	url: String,
	csrf: String,
}

// ---- Helpers ----

fn title_to_slug(title: &str) -> String {
	let mut slug = String::new();
	let mut last_was_hyphen = true; // prevent leading hyphen
	for c in title.chars() {
		if c.is_ascii_alphanumeric() {
			slug.push(c.to_ascii_lowercase());
			last_was_hyphen = false;
		} else if !last_was_hyphen {
			slug.push('-');
			last_was_hyphen = true;
		}
	}
	while slug.ends_with('-') {
		slug.pop();
	}
	slug.truncate(50);
	slug
}

fn valid_slug(s: &str) -> bool {
	!s.is_empty()
		&& s.len() <= 50
		&& s != "my-subs"
		&& s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn parse_subreddits(input: &str) -> Vec<String> {
	input
		.split(|c: char| c == '\n' || c == ',' || c == ' ')
		.map(|s| s.trim().to_string())
		.filter(|s| !s.is_empty() && s.len() <= 21 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
		.collect()
}

fn parse_channel_form(form: &HashMap<String, String>) -> Result<(String, String, ChannelRule), String> {
	let title = form.get("title").map(|s| s.trim().to_string()).unwrap_or_default();
	if title.is_empty() {
		return Err("Channel name is required.".to_string());
	}
	if title.len() > 100 {
		return Err("Channel name must be 100 characters or fewer.".to_string());
	}

	let slug_raw = form.get("slug").map(|s| s.trim().to_string()).unwrap_or_default();
	let slug = if slug_raw.is_empty() { title_to_slug(&title) } else { slug_raw };
	if !valid_slug(&slug) {
		return Err("Slug must be 1–50 lowercase alphanumeric characters and hyphens (not 'my-subs').".to_string());
	}

	let subscriptions = form.get("sources_subscriptions").map(|s| s == "on").unwrap_or(false);
	let subreddits = form.get("sources_subreddits").map(|s| parse_subreddits(s)).unwrap_or_default();

	if !subscriptions && subreddits.is_empty() {
		return Err("At least one source is required: enable subscriptions or add subreddits.".to_string());
	}

	let min_comments = form.get("gate_min_comments").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0).max(0);
	let min_score = form.get("gate_min_score").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
	let max_age_hours = form.get("gate_max_age_hours").and_then(|s| s.parse::<i64>().ok()).unwrap_or(72).max(0);
	let clusters = if form.get("presentation_clusters").map(|s| s.as_str()) == Some("off") { "off" } else { "on" };
	let density = form.get("presentation_density").map(|s| s.as_str()).unwrap_or("balanced").to_string();

	let rule = ChannelRule {
		sources: Sources { subscriptions, subreddits },
		filters: Filters {
			include_keywords: Vec::new(),
			exclude_keywords: Vec::new(),
			domains_allow: Vec::new(),
			domains_block: Vec::new(),
			media_types: vec!["self".into(), "link".into(), "image".into(), "video".into(), "gif".into(), "gallery".into()],
			nsfw: "hide".into(),
		},
		gates: Gates { min_comments, min_score, max_age_hours },
		presentation: Presentation { clusters: clusters.to_string(), density },
	};

	Ok((title, slug, rule))
}

// ---- Handlers ----

/// GET /channels
pub async fn channels_list(req: Request<Body>) -> Result<Response<Body>, String> {
	if !local_state_enabled() {
		return info(req, "Enable local state (REDLIB_ENABLE_LOCAL_STATE=on) to use channels.").await;
	}
	let prefs = Preferences::new(&req);
	let url = req.uri().to_string();
	let mut res = hyper::Response::new(Body::empty());
	let user_key = match require_user_key(&req, &mut res).await {
		Ok(k) => k,
		Err(msg) => return info(req, &msg).await,
	};

	let channels = if let State::Sqlite(store) = &*STATE {
		store.list_channels(&user_key).await.unwrap_or_default()
	} else {
		Vec::new()
	};

	let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);
	let body = ChannelsTemplate { prefs, channels, error: String::new(), url, csrf: csrf_tok };
	*res.body_mut() = Body::from(body.render().unwrap_or_default());
	*res.status_mut() = hyper::StatusCode::OK;
	res.headers_mut().insert("content-type", hyper::header::HeaderValue::from_static("text/html; charset=utf-8"));
	Ok(res)
}

/// POST /channels — create a new channel
pub async fn channels_create(mut req: Request<Body>) -> Result<Response<Body>, String> {
	if !local_state_enabled() {
		return info(req, "Enable local state (REDLIB_ENABLE_LOCAL_STATE=on) to use channels.").await;
	}
	let mut res = hyper::Response::new(Body::empty());
	let user_key = match require_user_key(&req, &mut res).await {
		Ok(k) => k,
		Err(msg) => return info(req, &msg).await,
	};

	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	match parse_channel_form(&form) {
		Ok((title, slug, rule)) => {
			if let State::Sqlite(store) = &*STATE {
				let rule_json = serde_json::to_string(&rule).map_err(|e| e.to_string())?;
				store.upsert_channel(&user_key, &slug, &title, &rule_json).await?;
			}
			Ok(redirect(&format!("/channels/{slug}")))
		}
		Err(msg) => {
			// Re-render list with error
			let prefs = Preferences::new(&req);
			let url = "/channels".to_string();
			let channels = if let State::Sqlite(store) = &*STATE {
				store.list_channels(&user_key).await.unwrap_or_default()
			} else {
				Vec::new()
			};
			let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);
			let body = ChannelsTemplate { prefs, channels, error: msg, url, csrf: csrf_tok };
			*res.body_mut() = Body::from(body.render().unwrap_or_default());
			*res.status_mut() = hyper::StatusCode::OK;
			res.headers_mut().insert("content-type", hyper::header::HeaderValue::from_static("text/html; charset=utf-8"));
			Ok(res)
		}
	}
}

/// GET /channels/:slug — edit form for a channel
pub async fn channels_edit(req: Request<Body>) -> Result<Response<Body>, String> {
	if !local_state_enabled() {
		return info(req, "Enable local state (REDLIB_ENABLE_LOCAL_STATE=on) to use channels.").await;
	}
	let slug = req.param("slug").unwrap_or_default();
	let prefs = Preferences::new(&req);
	let url = req.uri().to_string();
	let mut res = hyper::Response::new(Body::empty());
	let user_key = match require_user_key(&req, &mut res).await {
		Ok(k) => k,
		Err(msg) => return info(req, &msg).await,
	};

	let (title, rule) = if let State::Sqlite(store) = &*STATE {
		match store.get_channel(&user_key, &slug).await? {
			Some(row) => {
				let rule: ChannelRule = serde_json::from_str(&row.rule_json).unwrap_or_default();
				(row.title, rule)
			}
			None => return error(req, &format!("Channel '{}' not found.", slug)).await,
		}
	} else {
		return info(req, "Local state not available.").await;
	};

	let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);
	let body = ChannelEditTemplate { prefs, slug, title, rule, error: String::new(), url, csrf: csrf_tok };
	*res.body_mut() = Body::from(body.render().unwrap_or_default());
	*res.status_mut() = hyper::StatusCode::OK;
	res.headers_mut().insert("content-type", hyper::header::HeaderValue::from_static("text/html; charset=utf-8"));
	Ok(res)
}

/// POST /channels/:slug — update an existing channel
pub async fn channels_update(mut req: Request<Body>) -> Result<Response<Body>, String> {
	if !local_state_enabled() {
		return info(req, "Enable local state (REDLIB_ENABLE_LOCAL_STATE=on) to use channels.").await;
	}
	let slug = req.param("slug").unwrap_or_default();
	let mut res = hyper::Response::new(Body::empty());
	let user_key = match require_user_key(&req, &mut res).await {
		Ok(k) => k,
		Err(msg) => return info(req, &msg).await,
	};

	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	// Keep the existing slug, only update title and rule
	let title = form.get("title").map(|s| s.trim().to_string()).unwrap_or_default();
	if title.is_empty() {
		return error(req, "Channel name is required.").await;
	}

	let subscriptions = form.get("sources_subscriptions").map(|s| s == "on").unwrap_or(false);
	let subreddits = form.get("sources_subreddits").map(|s| parse_subreddits(s)).unwrap_or_default();
	let min_comments = form.get("gate_min_comments").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0).max(0);
	let min_score = form.get("gate_min_score").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
	let max_age_hours = form.get("gate_max_age_hours").and_then(|s| s.parse::<i64>().ok()).unwrap_or(72).max(0);
	let clusters = if form.get("presentation_clusters").map(|s| s.as_str()) == Some("off") { "off" } else { "on" };
	let density = form.get("presentation_density").map(|s| s.as_str()).unwrap_or("balanced").to_string();

	let rule = ChannelRule {
		sources: Sources { subscriptions, subreddits },
		filters: Filters {
			include_keywords: Vec::new(),
			exclude_keywords: Vec::new(),
			domains_allow: Vec::new(),
			domains_block: Vec::new(),
			media_types: vec!["self".into(), "link".into(), "image".into(), "video".into(), "gif".into(), "gallery".into()],
			nsfw: "hide".into(),
		},
		gates: Gates { min_comments, min_score, max_age_hours },
		presentation: Presentation { clusters: clusters.to_string(), density },
	};

	if let State::Sqlite(store) = &*STATE {
		let rule_json = serde_json::to_string(&rule).map_err(|e| e.to_string())?;
		store.upsert_channel(&user_key, &slug, &title, &rule_json).await?;
	}
	Ok(redirect(&format!("/reader/{slug}")))
}

/// POST /channels/:slug/delete — delete a channel
pub async fn channels_delete(mut req: Request<Body>) -> Result<Response<Body>, String> {
	if !local_state_enabled() {
		return info(req, "Enable local state (REDLIB_ENABLE_LOCAL_STATE=on) to use channels.").await;
	}
	let slug = req.param("slug").unwrap_or_default();
	let mut res = hyper::Response::new(Body::empty());
	let user_key = match require_user_key(&req, &mut res).await {
		Ok(k) => k,
		Err(msg) => return info(req, &msg).await,
	};

	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	if let State::Sqlite(store) = &*STATE {
		store.delete_channel(&user_key, &slug).await?;
	}
	Ok(redirect("/channels"))
}

async fn read_form(req: &mut Request<Body>) -> Result<HashMap<String, String>, String> {
	use hyper::body::to_bytes;
	let bytes = to_bytes(req.body_mut()).await.map_err(|e| e.to_string())?;
	let body = String::from_utf8(bytes.to_vec()).unwrap_or_default();
	Ok(serde_urlencoded::from_str(&body).unwrap_or_default())
}
