//! Post submission (text/link) to a subreddit.
//!
//! GET /r/:sub/submit — show form.
//! POST /r/:sub/submit — submit to Reddit (kind=self or link). Requires auth and submit scope.

use std::collections::HashMap;

use hyper::{Body, Request, Response};

use crate::auth::{update_session_cookie, validate_csrf_token, AuthContext};
use crate::client::authed_post;
use crate::server::RequestExt;
use crate::utils::{error, redirect, template, Preferences};
use askama::Template;

const MAX_BODY_SIZE: usize = 64 * 1024;
const MAX_TITLE_LENGTH: usize = 300;
const MAX_TEXT_LENGTH: usize = 40000;

#[derive(Template)]
#[template(path = "submit.html")]
struct SubmitTemplate {
	sub: String,
	prefs: Preferences,
	error: String,
	url: String,
}

/// GET /r/:sub/submit — show submission form.
pub async fn get(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);
	if !auth.is_authenticated() {
		return Ok(redirect("/login"));
	}
	let sub = req.param("sub").unwrap_or_default();
	if sub.is_empty() || sub == "all" || sub == "popular" || sub.contains('+') {
		return error(req, "Cannot submit to this feed.").await;
	}
	let url = req.uri().to_string();
	Ok(template(&SubmitTemplate {
		sub,
		prefs: Preferences::new(&req),
		error: String::new(),
		url,
	}))
}

/// POST /r/:sub/submit — submit post to Reddit.
pub async fn post(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);
	if !auth.is_authenticated() {
		return Err("You must be logged in to submit.".to_string());
	}
	let sub = req.param("sub").unwrap_or_default();
	if sub.is_empty() || sub == "all" || sub == "popular" || sub.contains('+') {
		return Err("Cannot submit to this feed.".to_string());
	}

	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large.".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();

	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
	validate_csrf_token(&auth, submitted_csrf)?;

	let kind = form.get("kind").map(|s| s.as_str()).unwrap_or("self").to_lowercase();
	let kind = match kind.as_str() {
		"link" => "link",
		_ => "self",
	};
	let title = form.get("title").map(|s| s.trim()).unwrap_or("");
	let text = form.get("text").map(|s| s.as_str()).unwrap_or("");
	let url_link = form.get("url").map(|s| s.trim()).unwrap_or("");

	if title.is_empty() {
		return Err("Title is required.".to_string());
	}
	if title.len() > MAX_TITLE_LENGTH {
		return Err(format!("Title must be at most {} characters.", MAX_TITLE_LENGTH));
	}
	if kind == "link" {
		if url_link.is_empty() {
			return Err("URL is required for link posts.".to_string());
		}
		if !url_link.starts_with("http://") && !url_link.starts_with("https://") {
			return Err("URL must start with http:// or https://.".to_string());
		}
	} else if text.len() > MAX_TEXT_LENGTH {
		return Err(format!("Text must be at most {} characters.", MAX_TEXT_LENGTH));
	}

	let mut body_str = format!(
		"api_type=json&kind={}&sr={}&title={}",
		kind,
		percent_encoding::utf8_percent_encode(&sub, percent_encoding::NON_ALPHANUMERIC),
		percent_encoding::utf8_percent_encode(title, percent_encoding::NON_ALPHANUMERIC),
	);
	if kind == "link" {
		body_str.push_str(&format!("&url={}", percent_encoding::utf8_percent_encode(url_link, percent_encoding::NON_ALPHANUMERIC)));
	} else {
		body_str.push_str(&format!("&text={}", percent_encoding::utf8_percent_encode(text, percent_encoding::NON_ALPHANUMERIC)));
	}

	let (value, session_updated) = authed_post("/api/submit".to_string(), body_str, &auth).await?;

	// Reddit returns { "json": { "errors": [], "data": { "url": "...", ... } } } on success
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
	let return_to = form.get("return_to").map(|s| s.as_str()).unwrap_or("/");
	let return_to = if return_to.starts_with('/') && !return_to.starts_with("//") { return_to } else { "/" };
	let redirect_url: String = if let Some(data) = json.and_then(|j| j.get("data")).and_then(|d| d.get("url")).and_then(|u| u.as_str()) {
		if data.starts_with("http") {
			return_to.to_string()
		} else {
			data.to_string()
		}
	} else {
		format!("/r/{}", sub)
	};

	let mut res = redirect(&redirect_url);
	if let Some(s) = session_updated {
		update_session_cookie(&mut res, &s);
	}
	Ok(res)
}
