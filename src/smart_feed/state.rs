use super::csrf;
use super::session::{ensure_sid, local_state_enabled, require_user_key};
use crate::utils::{error, redirect};
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

fn valid_mute_scope(scope: &str) -> bool {
	scope == "global" || scope.starts_with("channel:")
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
	let scope = form.get("scope").map(|s| s.as_str()).unwrap_or("global");
	let scope = if valid_mute_scope(scope) { scope } else { "global" };
	if let State::Sqlite(store) = &*STATE {
		store.add_mute(&user_key, scope, "keyword", &pattern).await?;
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
	let scope = form.get("scope").map(|s| s.as_str()).unwrap_or("global");
	let scope = if valid_mute_scope(scope) { scope } else { "global" };
	if let State::Sqlite(store) = &*STATE {
		store.add_mute(&user_key, scope, "domain", &pattern).await?;
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
	let scope = form.get("scope").map(|s| s.as_str()).unwrap_or("global");
	let scope = if valid_mute_scope(scope) { scope } else { "global" };
	if let State::Sqlite(store) = &*STATE {
		store.add_mute(&user_key, scope, "subreddit", &pattern).await?;
	}
	Ok(redirect_back(&req))
}

pub async fn action_archive(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	let post_id = form.get("post_id").cloned().unwrap_or_default();
	if post_id.is_empty() {
		return error(req, "Missing post_id").await;
	}
	if let State::Sqlite(store) = &*STATE {
		store.set_archived(&user_key, &post_id, true).await?;
	}
	Ok(redirect_back(&req))
}

pub async fn action_unarchive(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	let post_id = form.get("post_id").cloned().unwrap_or_default();
	if post_id.is_empty() {
		return error(req, "Missing post_id").await;
	}
	if let State::Sqlite(store) = &*STATE {
		store.set_archived(&user_key, &post_id, false).await?;
	}
	Ok(redirect_back(&req))
}

pub async fn action_mark_all_read(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	if let State::Sqlite(store) = &*STATE {
		store.mark_all_read(&user_key).await?;
	}
	Ok(redirect_back(&req))
}

/// GET /api/reader/unread_count — returns JSON {"count": N}
pub async fn api_unread_count(req: Request<Body>) -> Result<Response<Body>, String> {
	let mut fake_res = Response::new(Body::empty());
	let count = if local_state_enabled() {
		if let Some(user_key) = ensure_sid(&req, &mut fake_res) {
			if let State::Sqlite(store) = &*STATE {
				store.count_unread(&user_key).await.unwrap_or(0)
			} else {
				0
			}
		} else {
			0
		}
	} else {
		0
	};
	let json = format!("{{\"count\":{count}}}");
	let mut res = Response::new(Body::from(json));
	res.headers_mut().insert("content-type", hyper::header::HeaderValue::from_static("application/json"));
	Ok(res)
}

/// GET /action/open?post_id=X&url=/r/sub/comments/...
/// Marks the post read then redirects to the post URL.
/// URL must be a relative path (starts with /) to prevent open redirect.
pub async fn action_open(req: Request<Body>) -> Result<Response<Body>, String> {
	let query: std::collections::HashMap<String, String> = serde_urlencoded::from_str(req.uri().query().unwrap_or("")).unwrap_or_default();

	let post_id = query.get("post_id").cloned().unwrap_or_default();
	let dest = query.get("url").cloned().unwrap_or_else(|| "/".to_string());

	// Only allow relative paths
	let dest = if dest.starts_with('/') { dest } else { "/".to_string() };

	// Mark read if local state enabled and user has a session
	if local_state_enabled() && !post_id.is_empty() {
		let mut fake_res = Response::new(Body::empty());
		if let Some(user_key) = ensure_sid(&req, &mut fake_res) {
			if let State::Sqlite(store) = &*STATE {
				let _ = store.set_read(&user_key, &post_id, true).await;
			}
		}
	}

	Ok(redirect(&dest))
}
