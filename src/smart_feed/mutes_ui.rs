use super::csrf;
use super::session::{ensure_sid, local_state_enabled, require_user_key};
use crate::state::{MuteRule, State, STATE};
use crate::utils::{info, Preferences};
use askama::Template;
use hyper::{Body, Request, Response};
use serde::{Deserialize, Serialize};

#[derive(Template)]
#[template(path = "mutes.html")]
struct MutesTemplate {
	prefs: Preferences,
	mutes: Vec<MuteRule>,
	csrf: String,
	url: String,
}

pub async fn mutes_list(req: Request<Body>) -> Result<Response<Body>, String> {
	let prefs = Preferences::new(&req);
	let url = req.uri().to_string();
	let mut res = Response::new(Body::empty());

	let user_key = ensure_sid(&req, &mut res);
	let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);

	let mutes = if let (Some(ref key), true) = (user_key, local_state_enabled()) {
		if let State::Sqlite(store) = &*STATE {
			store.list_all_mutes(key).await.unwrap_or_default()
		} else {
			Vec::new()
		}
	} else {
		Vec::new()
	};

	if !local_state_enabled() {
		return info(req, "Enable local state (REDLIB_ENABLE_LOCAL_STATE=on) to manage mutes.").await;
	}

	let body = MutesTemplate { prefs, mutes, csrf: csrf_tok, url };
	*res.body_mut() = Body::from(body.render().unwrap_or_default());
	*res.status_mut() = hyper::StatusCode::OK;
	res.headers_mut().insert("content-type", hyper::header::HeaderValue::from_static("text/html; charset=utf-8"));
	Ok(res)
}

async fn read_form(req: &mut Request<Body>) -> Result<std::collections::HashMap<String, String>, String> {
	use hyper::body::to_bytes;
	let bytes = to_bytes(req.body_mut()).await.map_err(|e| e.to_string())?;
	let body = String::from_utf8(bytes.to_vec()).unwrap_or_default();
	Ok(serde_urlencoded::from_str(&body).unwrap_or_default())
}

pub async fn action_unmute(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	let mute_id: i64 = form
		.get("mute_id")
		.and_then(|s| s.parse().ok())
		.ok_or_else(|| "Missing or invalid mute_id".to_string())?;

	if let State::Sqlite(store) = &*STATE {
		store.delete_mute(&user_key, mute_id).await?;
	}

	Ok(crate::utils::redirect(
		form.get("back").map(|s| s.as_str()).unwrap_or("/mutes"),
	))
}

#[derive(Serialize, Deserialize)]
struct MuteExportEntry {
	rule_type: String,
	pattern: String,
}

pub async fn action_export_mutes(req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = ensure_sid(&req, &mut res);
	if !local_state_enabled() {
		return info(req, "Local state disabled.").await;
	}
	let key = match user_key {
		Some(k) => k,
		None => return info(req, "No session.").await,
	};

	let pairs = if let State::Sqlite(store) = &*STATE {
		store.export_mutes(&key).await.unwrap_or_default()
	} else {
		Vec::new()
	};

	let entries: Vec<MuteExportEntry> = pairs
		.into_iter()
		.map(|(rule_type, pattern)| MuteExportEntry { rule_type, pattern })
		.collect();

	let json = serde_json::to_string_pretty(&entries).map_err(|e| e.to_string())?;
	let mut out = Response::new(Body::from(json));
	out.headers_mut().insert("content-type", hyper::header::HeaderValue::from_static("application/json"));
	out.headers_mut().insert(
		"content-disposition",
		hyper::header::HeaderValue::from_static("attachment; filename=\"mutes.json\""),
	);
	Ok(out)
}

pub async fn action_import_mutes(mut req: Request<Body>) -> Result<Response<Body>, String> {
	let mut res = Response::new(Body::empty());
	let user_key = require_user_key(&req, &mut res).await?;
	let form = read_form(&mut req).await?;
	csrf::verify_csrf(&req, form.get("csrf").map(|s| s.as_str()).unwrap_or_default())?;

	let json_str = form.get("json").cloned().unwrap_or_default();
	let entries: Vec<MuteExportEntry> = serde_json::from_str(&json_str).map_err(|e| format!("Invalid JSON: {e}"))?;

	if let State::Sqlite(store) = &*STATE {
		for e in &entries {
			let rule_type = e.rule_type.as_str();
			if !["keyword", "domain", "subreddit"].contains(&rule_type) {
				continue;
			}
			let _ = store.add_mute(&user_key, "global", rule_type, &e.pattern).await;
		}
	}

	Ok(crate::utils::redirect("/mutes"))
}
