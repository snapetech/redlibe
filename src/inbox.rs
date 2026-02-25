//! Inbox and private messages.
//!
//! GET /inbox — list messages (inbox/unread).
//! GET /inbox/compose — compose form.
//! POST /inbox/compose — send PM.

use std::collections::HashMap;

use hyper::{Body, Request, Response};
use serde_json::Value;

use crate::auth::{update_session_cookie, validate_csrf_token, AuthContext};
use crate::client::{authed_json, authed_post};
use crate::utils::{param, redirect, template, Preferences};
use askama::Template;

#[derive(Clone)]
pub struct InboxMessage {
	pub id: String,
	pub fullname: String,
	pub subject: String,
	pub author: String,
	pub body_preview: String,
	pub created: String,
	pub new: bool,
	pub kind: String,
}

fn val(v: &Value, key: &str) -> String {
	v.get(key).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

fn parse_inbox_listing(json: &Value) -> Vec<InboxMessage> {
	let children = match json["data"]["children"].as_array() {
		Some(a) => a,
		None => return vec![],
	};
	let mut out = Vec::with_capacity(children.len());
	for item in children {
		let data = &item["data"];
		let created_utc = data["created_utc"].as_f64().unwrap_or(0.0);
		let created = crate::utils::time(created_utc).1;
		let body_html = val(data, "body_html");
		let body_plain: String = body_html
			.replace("<p>", " ")
			.replace("</p>", " ")
			.replace("<br>", " ")
			.replace("</br>", " ")
			.replace('<', " ")
			.replace('>', " ");
		let body_preview: String = body_plain.split_whitespace().take(30).collect::<Vec<_>>().join(" ");
		let body_preview = if body_preview.len() > 150 {
			format!("{}...", &body_preview[..147])
		} else {
			body_preview
		};
		out.push(InboxMessage {
			id: val(data, "id"),
			fullname: val(data, "name"),
			subject: val(data, "subject"),
			author: val(data, "author"),
			body_preview,
			created,
			new: data["new"].as_bool().unwrap_or(false),
			kind: item["kind"].as_str().unwrap_or("").to_string(),
		});
	}
	out
}

#[derive(Template)]
#[template(path = "inbox.html")]
struct InboxTemplate {
	prefs: Preferences,
	messages: Vec<InboxMessage>,
	after: String,
	url: String,
	tab: String,
}

#[derive(Template)]
#[template(path = "inbox_compose.html")]
struct InboxComposeTemplate {
	prefs: Preferences,
	error: String,
	to: String,
	subject: String,
	url: String,
}

const MAX_BODY_SIZE: usize = 64 * 1024;
const MAX_SUBJECT_LENGTH: usize = 100;

/// GET /inbox — list inbox or unread messages.
pub async fn list(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);
	if !auth.is_authenticated() {
		return Ok(redirect("/login"));
	}
	let url = req.uri().to_string();
	let tab = param(&url, "tab").unwrap_or_else(|| "inbox".to_string());
	let path = match tab.as_str() {
		"unread" => "/message/unread.json",
		"sent" => "/message/sent.json",
		_ => "/message/inbox.json",
	};
	let query = req.uri().query().unwrap_or_default();
	let path = format!("{}?limit=25&raw_json=1&{}", path, query);
	let (json, session_updated) = authed_json(path, false, &auth).await?;
	let messages = parse_inbox_listing(&json);
	let after = json["data"]["after"].as_str().unwrap_or("").to_string();
	let mut res = template(&InboxTemplate {
		prefs: Preferences::new(&req),
		messages,
		after,
		url,
		tab: tab.clone(),
	});
	if let Some(s) = session_updated {
		update_session_cookie(&mut res, &s);
	}
	Ok(res)
}

/// GET /inbox/compose — show compose form.
pub async fn compose_get(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);
	if !auth.is_authenticated() {
		return Ok(redirect("/login"));
	}
	let url = req.uri().to_string();
	let to = param(&url, "to").unwrap_or_default();
	Ok(template(&InboxComposeTemplate {
		prefs: Preferences::new(&req),
		error: String::new(),
		to,
		subject: String::new(),
		url,
	}))
}

/// POST /inbox/compose — send a private message.
pub async fn compose_post(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);
	if !auth.is_authenticated() {
		return Err("You must be logged in to send a message.".to_string());
	}
	let prefs = Preferences::new(&req);
	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large.".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
	validate_csrf_token(&auth, submitted_csrf)?;
	let to = form.get("to").map(|s| s.trim()).unwrap_or("");
	let subject = form.get("subject").map(|s| s.trim()).unwrap_or("");
	let text = form.get("text").map(|s| s.as_str()).unwrap_or("");
	if to.is_empty() {
		return Err("Recipient is required.".to_string());
	}
	if subject.len() > MAX_SUBJECT_LENGTH {
		return Err(format!("Subject must be at most {} characters.", MAX_SUBJECT_LENGTH));
	}
	if text.is_empty() {
		return Err("Message text is required.".to_string());
	}
	let body_str = format!(
		"api_type=json&to={}&subject={}&text={}",
		percent_encoding::utf8_percent_encode(to, percent_encoding::NON_ALPHANUMERIC),
		percent_encoding::utf8_percent_encode(subject, percent_encoding::NON_ALPHANUMERIC),
		percent_encoding::utf8_percent_encode(text, percent_encoding::NON_ALPHANUMERIC),
	);
	let (value, session_updated) = authed_post("/api/compose".to_string(), body_str, &auth).await?;
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
			let url = "/inbox/compose".to_string();
			let mut res = template(&InboxComposeTemplate {
				prefs,
				error: format!("Reddit: {msg}"),
				to: to.to_string(),
				subject: subject.to_string(),
				url,
			});
			if let Some(s) = session_updated {
				update_session_cookie(&mut res, &s);
			}
			return Ok(res);
		}
	}
	let mut res = redirect("/inbox");
	if let Some(s) = session_updated {
		update_session_cookie(&mut res, &s);
	}
	Ok(res)
}

/// POST /inbox/read — mark a message as read.
/// Form: id=t1_xxx (fullname of the message)
pub async fn read_message(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);
	if !auth.is_authenticated() {
		return Err("You must be logged in to read messages.".to_string());
	}

	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large.".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();

	let id = form.get("id").map(|s| s.trim()).unwrap_or("");
	if id.is_empty() {
		return Err("Message ID is required.".to_string());
	}

	let body_str = format!("id={}", percent_encoding::utf8_percent_encode(id, percent_encoding::NON_ALPHANUMERIC));
	let (_, session_updated) = authed_post("/api/read_message".to_string(), body_str, &auth).await?;

	let mut res = redirect("/inbox");
	if let Some(s) = session_updated {
		update_session_cookie(&mut res, &s);
	}
	Ok(res)
}

/// POST /inbox/read-all — mark all messages as read.
pub async fn read_all(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);
	if !auth.is_authenticated() {
		return Err("You must be logged in to read messages.".to_string());
	}

	let (_, session_updated) = authed_post("/api/read_all_messages".to_string(), String::new(), &auth).await?;

	let mut res = redirect("/inbox");
	if let Some(s) = session_updated {
		update_session_cookie(&mut res, &s);
	}
	Ok(res)
}
