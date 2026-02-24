//! Edit or delete own comment or post.
//!
//! POST /edit — form: thing_id (fullname t1_/t3_), action (edit|delete), text (for edit), csrf_token, return_to.

use std::collections::HashMap;

use hyper::{Body, Request, Response};

use crate::auth::{update_session_cookie, validate_csrf_token, AuthContext};
use crate::client::authed_post;
use crate::utils::redirect;

const MAX_BODY_SIZE: usize = 64 * 1024;
const MAX_TEXT_LENGTH: usize = 40000;

fn safe_return_to(raw: &str) -> &str {
	if raw.starts_with('/') && !raw.starts_with("//") {
		raw
	} else {
		"/"
	}
}

/// POST /edit — edit (POST /api/editusertext) or delete (POST /api/del) a comment or self-post.
pub async fn submit(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);
	if !auth.is_authenticated() {
		return Err("You must be logged in to edit or delete.".to_string());
	}

	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large.".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes)
		.map(|(k, v)| (k.into_owned(), v.into_owned()))
		.collect();

	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
	validate_csrf_token(&auth, submitted_csrf)?;

	let thing_id = form.get("thing_id").map(|s| s.trim()).unwrap_or("");
	if thing_id.is_empty() {
		return Err("Missing thing_id.".to_string());
	}
	if !thing_id.starts_with("t1_") && !thing_id.starts_with("t3_") {
		return Err("Invalid thing_id: must be t1_ (comment) or t3_ (post).".to_string());
	}

	let action = form.get("action").map(|s| s.as_str()).unwrap_or("edit").to_lowercase();
	let action = if action == "delete" { "delete" } else { "edit" };

	let return_to = form.get("return_to").map(|s| s.as_str()).unwrap_or("/");
	let return_to = safe_return_to(return_to);

	if action == "delete" {
		let body_str = format!(
			"id={}",
			percent_encoding::utf8_percent_encode(thing_id, percent_encoding::NON_ALPHANUMERIC)
		);
		let (value, session_updated) = authed_post("/api/del".to_string(), body_str, &auth).await?;
		if value.get("error").and_then(|e| e.as_i64()).is_some() {
			return Err(format!(
				"Reddit: {} | {}",
				value["reason"].as_str().unwrap_or(""),
				value["message"].as_str().unwrap_or("")
			));
		}
		let mut res = redirect(return_to);
		if let Some(s) = session_updated {
			update_session_cookie(&mut res, &s);
		}
		return Ok(res);
	}

	let text = form.get("text").map(|s| s.as_str()).unwrap_or("");
	if text.len() > MAX_TEXT_LENGTH {
		return Err(format!("Text must be at most {} characters.", MAX_TEXT_LENGTH));
	}
	let body_str = format!(
		"api_type=json&thing_id={}&text={}",
		percent_encoding::utf8_percent_encode(thing_id, percent_encoding::NON_ALPHANUMERIC),
		percent_encoding::utf8_percent_encode(text, percent_encoding::NON_ALPHANUMERIC),
	);
	let (value, session_updated) = authed_post("/api/editusertext".to_string(), body_str, &auth).await?;
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
	let mut res = redirect(return_to);
	if let Some(s) = session_updated {
		update_session_cookie(&mut res, &s);
	}
	Ok(res)
}
