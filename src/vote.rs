//! Voting handler for redlib-extended.
//!
//! `POST /vote` — upvote, downvote, or un-vote a post or comment.
//!
//! Form fields:
//!   - `thing_id`   — full Reddit thing ID, e.g. `t3_abc123` (post) or `t1_xyz789` (comment)
//!   - `dir`        — vote direction: `1` (up), `0` (un-vote), `-1` (down)
//!   - `csrf_token` — must match the session's stored CSRF token
//!   - `return_to`  — URL to redirect back to after voting (must be a relative path; defaults to `/`)
//!
//! Requires an active `AuthContext::UserSession` or `AuthContext::RawBearer`.
//! Anonymous requests receive a 403.

use std::collections::HashMap;

use hyper::{Body, Request, Response};

use crate::auth::{update_session_cookie, validate_csrf_token, AuthContext};
use crate::client::authed_post;
use crate::utils::redirect;

/// Maximum form body size accepted by vote/save handlers (64 KiB).
const MAX_BODY_SIZE: usize = 64 * 1024;

/// Validate and sanitize a `return_to` redirect target.
///
/// Only allows relative paths starting with `/` (but not `//`, which could
/// be interpreted as a protocol-relative URL and used for open redirect attacks).
fn safe_return_to(raw: &str) -> &str {
	if raw.starts_with('/') && !raw.starts_with("//") {
		raw
	} else {
		"/"
	}
}

/// `POST /vote` — submit a vote to Reddit.
pub async fn submit(req: Request<Body>) -> Result<Response<Body>, String> {
	// Resolve auth before consuming the body
	let auth = AuthContext::from_request(&req);

	// Reject anonymous users immediately
	if !auth.is_authenticated() {
		return Err("You must be logged in to vote".to_string());
	}

	// Read and parse the form body (reject oversized requests)
	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes)
		.map(|(k, v)| (k.into_owned(), v.into_owned()))
		.collect();

	// Validate CSRF
	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
	validate_csrf_token(&auth, submitted_csrf)?;

	// Extract required fields
	let thing_id = form.get("thing_id").ok_or("Missing 'thing_id' in vote form")?;
	let dir_str = form.get("dir").ok_or("Missing 'dir' in vote form")?;
	let raw_return_to = form.get("return_to").map(|s| s.as_str()).unwrap_or("/");
	let return_to = safe_return_to(raw_return_to);

	// Validate dir: must be 1, 0, or -1
	let dir: i8 = dir_str.parse().map_err(|_| format!("Invalid vote direction '{dir_str}' — must be 1, 0, or -1"))?;
	if ![-1i8, 0, 1].contains(&dir) {
		return Err(format!("Invalid vote direction {dir} — must be 1, 0, or -1"));
	}

	// Validate thing_id prefix (t1_ comment, t3_ link/post)
	if !thing_id.starts_with("t1_") && !thing_id.starts_with("t3_") {
		return Err(format!("Invalid thing_id '{thing_id}' — expected t1_ or t3_ prefix"));
	}

	// Submit vote to Reddit
	let body_str = format!(
		"id={}&dir={}&rank=10",
		percent_encoding::utf8_percent_encode(thing_id, percent_encoding::NON_ALPHANUMERIC),
		dir,
	);

	let (_, session_updated) = authed_post("/api/vote".to_string(), body_str, &auth).await?;

	let mut res = redirect(return_to);
	if let Some(s) = session_updated {
		update_session_cookie(&mut res, &s);
	}
	Ok(res)
}

/// `POST /r/:sub/comments/:id/save` — save or unsave a post/comment.
pub async fn save(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);

	if !auth.is_authenticated() {
		return Err("You must be logged in to save posts".to_string());
	}

	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes)
		.map(|(k, v)| (k.into_owned(), v.into_owned()))
		.collect();

	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
	validate_csrf_token(&auth, submitted_csrf)?;

	let thing_id = form.get("thing_id").ok_or("Missing 'thing_id'")?;
	let action = form.get("action").map(|s| s.as_str()).unwrap_or("save");
	let raw_return_to = form.get("return_to").map(|s| s.as_str()).unwrap_or("/");
	let return_to = safe_return_to(raw_return_to);

	let endpoint = if action == "unsave" { "/api/unsave" } else { "/api/save" };
	let body_str = format!("id={}", percent_encoding::utf8_percent_encode(thing_id, percent_encoding::NON_ALPHANUMERIC));

	let (_, session_updated) = authed_post(endpoint.to_string(), body_str, &auth).await?;

	let mut res = redirect(return_to);
	if let Some(s) = session_updated {
		update_session_cookie(&mut res, &s);
	}
	Ok(res)
}
