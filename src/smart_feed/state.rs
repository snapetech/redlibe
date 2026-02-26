use super::csrf;
use crate::config::get_setting;
use crate::server::{RequestExt, ResponseExt};
use crate::utils::{error, info};
use cookie::Cookie;
use hyper::{Body, Request, Response};

use crate::state::{State, STATE};

fn redirect_back(req: &Request<Body>) -> Response<Body> {
	let back = req
		.headers()
		.get("Referer")
		.and_then(|v| v.to_str().ok())
		.unwrap_or("/reader/my-subs?lens=reader&preset=digest15");
	crate::utils::redirect(back)
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

async fn require_user_key(req: &Request<Body>, res: &mut Response<Body>) -> Result<String, String> {
	ensure_sid(req, res).ok_or_else(|| "Local state disabled".to_string())
}

async fn read_form(req: &mut Request<Body>) -> Result<std::collections::HashMap<String, String>, String> {
	use hyper::body::to_bytes;
	let bytes = to_bytes(req.body_mut()).await.map_err(|e| e.to_string())?;
	let body = String::from_utf8(bytes.to_vec()).unwrap_or_default();
	Ok(serde_urlencoded::from_str(&body).unwrap_or_default())
}

pub async fn action_mark_read(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	let post_id = form.get("post_id").cloned().unwrap_or_default();
	if post_id.is_empty() {
		return error(req, "Missing post_id").await;
	}

	if let State::Sqlite(store) = &*STATE {
		store.set_read(&user_key, &post_id, true).await?;
	}
	Ok(redirect_back(&req))
}

pub async fn action_mark_unread(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	let post_id = form.get("post_id").cloned().unwrap_or_default();
	if post_id.is_empty() {
		return error(req, "Missing post_id").await;
	}
	if let State::Sqlite(store) = &*STATE {
		store.set_read(&user_key, &post_id, false).await?;
	}
	Ok(redirect_back(&req))
}

pub async fn action_save(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	let post_id = form.get("post_id").cloned().unwrap_or_default();
	if post_id.is_empty() {
		return error(req, "Missing post_id").await;
	}
	if let State::Sqlite(store) = &*STATE {
		store.set_saved(&user_key, &post_id, true).await?;
	}
	Ok(redirect_back(&req))
}

pub async fn action_unsave(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	let post_id = form.get("post_id").cloned().unwrap_or_default();
	if post_id.is_empty() {
		return error(req, "Missing post_id").await;
	}
	if let State::Sqlite(store) = &*STATE {
		store.set_saved(&user_key, &post_id, false).await?;
	}
	Ok(redirect_back(&req))
}

pub async fn action_mute_keyword(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	let pattern = form.get("pattern").cloned().unwrap_or_default();
	if pattern.is_empty() {
		return error(req, "Missing pattern").await;
	}
	if let State::Sqlite(store) = &*STATE {
		store.add_mute(&user_key, "global", "keyword", &pattern).await?;
	}
	Ok(redirect_back(&req))
}

pub async fn action_mute_domain(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	let pattern = form.get("pattern").cloned().unwrap_or_default();
	if pattern.is_empty() {
		return error(req, "Missing pattern").await;
	}
	if let State::Sqlite(store) = &*STATE {
		store.add_mute(&user_key, "global", "domain", &pattern).await?;
	}
	Ok(redirect_back(&req))
}

pub async fn action_mute_subreddit(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	let pattern = form.get("pattern").cloned().unwrap_or_default();
	if pattern.is_empty() {
		return error(req, "Missing pattern").await;
	}
	if let State::Sqlite(store) = &*STATE {
		store.add_mute(&user_key, "global", "subreddit", &pattern).await?;
	}
	Ok(redirect_back(&req))
}
