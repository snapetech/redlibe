//! Comment submission (reply to post or comment).
//!
//! `POST /comment` — submit a reply. Form: parent (fullname t1_ or t3_), text, csrf_token, return_to.
//! Requires auth and `submit` scope.

use std::collections::HashMap;

use hyper::{Body, Request, Response};

use crate::auth::{update_session_cookie, validate_csrf_token, AuthContext};
use crate::client::authed_post;
use crate::utils::redirect;

const MAX_BODY_SIZE: usize = 64 * 1024;
const MAX_COMMENT_LENGTH: usize = 10000;

fn safe_return_to(raw: &str) -> &str {
	if raw.starts_with('/') && !raw.starts_with("//") {
		raw
	} else {
		"/"
	}
}

/// `POST /comment` — submit a comment or reply to Reddit.
pub async fn submit(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);

	if !auth.is_authenticated() {
		return Err("You must be logged in to reply".to_string());
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

	let parent = form.get("parent").ok_or("Missing 'parent' (thing ID)")?.trim();
	let text = form.get("text").ok_or("Missing 'text'")?.trim();

	if !parent.starts_with("t1_") && !parent.starts_with("t3_") {
		return Err("Invalid parent: must be t1_ (comment) or t3_ (post)".to_string());
	}
	if text.is_empty() {
		return Err("Comment text cannot be empty".to_string());
	}
	if text.len() > MAX_COMMENT_LENGTH {
		return Err(format!("Comment too long (max {} characters)", MAX_COMMENT_LENGTH));
	}

	let body_str = format!(
		"api_type=json&parent={}&text={}",
		percent_encoding::utf8_percent_encode(parent, percent_encoding::NON_ALPHANUMERIC),
		percent_encoding::utf8_percent_encode(text, percent_encoding::NON_ALPHANUMERIC),
	);

	let (value, session_updated) = authed_post("/api/comment".to_string(), body_str, &auth).await?;

	// Reddit returns { "json": { "errors": [] } } on success, or errors in "errors" array
	let json = value.get("json").and_then(|j| j.as_object());
	let errors = json.and_then(|j| j.get("errors").and_then(|e| e.as_array()));
	if let Some(errs) = errors {
		if !errs.is_empty() {
			let msg = errs
				.iter()
				.filter_map(|e| e.as_array())
				.filter_map(|a| a.get(1).and_then(|s| s.as_str()))
				.next()
				.unwrap_or("Unknown error");
			return Err(format!("Reddit: {msg}"));
		}
	}

	let raw_return_to = form.get("return_to").map(|s| s.as_str()).unwrap_or("/");
	let return_to = safe_return_to(raw_return_to);

	let mut res = redirect(return_to);
	if let Some(s) = session_updated {
		update_session_cookie(&mut res, &s);
	}
	Ok(res)
}
