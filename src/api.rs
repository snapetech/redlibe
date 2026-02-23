//! REST API layer for redlib-extended.
//!
//! All endpoints are prefixed with `/api/v1`.
//!
//! Authentication:
//!   - Header: `Authorization: Bearer <access_token>` (takes priority over cookie)
//!   - Cookie: `rl_session` encrypted session (fallback)
//!   - Anonymous requests return data without user-specific fields.
//!
//! Error responses are JSON: `{ "error": "<message>", "code": <http_status> }`

use std::collections::HashMap;

use hyper::{Body, Request, Response};
use serde_json::{json, Value};

use crate::auth::AuthContext;
use crate::client::{authed_json, authed_post, json as anon_json};
use crate::server::RequestExt;
use url;

/// Maximum JSON/form body size accepted by API POST handlers (64 KiB).
const MAX_BODY_SIZE: usize = 64 * 1024;

/// Allowed query parameter names forwarded to Reddit to prevent parameter injection.
const ALLOWED_QUERY_PARAMS: &[&str] = &["sort", "t", "after", "before", "limit", "count", "depth", "showmore"];

// ----- Helpers -----

/// Build a JSON response with the given status code.
fn json_response(status: u16, body: Value) -> Result<Response<Body>, String> {
	let bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
	Response::builder()
		.status(status)
		.header("Content-Type", "application/json; charset=utf-8")
		.header("Cache-Control", "no-store")
		.body(Body::from(bytes))
		.map_err(|e| e.to_string())
}

/// Build a JSON error response.
fn json_error(status: u16, message: &str) -> Result<Response<Body>, String> {
	json_response(status, json!({ "error": message, "code": status }))
}

/// Resolve the auth context for an API request, checking the `Authorization`
/// header before falling back to the cookie session.
fn api_auth(req: &Request<Body>) -> AuthContext {
	// Check for `Authorization: Bearer <token>` header first
	if let Some(auth_header) = req.headers().get("Authorization") {
		if let Ok(value) = auth_header.to_str() {
			if let Some(token) = value.strip_prefix("Bearer ") {
				if !token.is_empty() {
					return AuthContext::RawBearer(token.to_string());
				}
			}
		}
	}
	// Fall back to cookie-based session
	AuthContext::from_request(req)
}

// ----- Endpoints -----

/// `GET /api/v1/r/:sub` — subreddit listing as JSON.
///
/// Passes through the Reddit listing with no transformation.
/// Supports query params: `sort`, `t`, `after`, `before`, `limit`.
pub async fn subreddit_listing(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = api_auth(&req);
	let sub = req.param("sub").unwrap_or_else(|| "popular".to_string());

	// Forward only whitelisted query params to prevent injection
	let query = req.uri().query().unwrap_or("");
	let filtered: Vec<String> = url::form_urlencoded::parse(query.as_bytes())
		.filter(|(k, _)| ALLOWED_QUERY_PARAMS.contains(&k.as_ref()))
		.map(|(k, v)| format!("{}={}", k, percent_encoding::utf8_percent_encode(&v, percent_encoding::NON_ALPHANUMERIC)))
		.collect();
	let qs = if filtered.is_empty() {
		"?raw_json=1".to_string()
	} else {
		format!("?{}&raw_json=1", filtered.join("&"))
	};
	let path = format!("/r/{sub}/hot.json{qs}");

	let data = match auth.bearer_token() {
		Some(_) => authed_json(path, false, &auth).await,
		None => anon_json(path, false).await,
	}
	.map_err(|e| e)?;

	json_response(200, data)
}

/// `GET /api/v1/r/:sub/comments/:id` — post + comments as JSON.
pub async fn post_comments(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = api_auth(&req);
	let sub = req.param("sub").unwrap_or_default();
	let id = req.param("id").unwrap_or_default();
	let query = req.uri().query().unwrap_or("");
	let qs = if query.is_empty() {
		"?raw_json=1".to_string()
	} else {
		format!("?{query}&raw_json=1")
	};
	let path = format!("/r/{sub}/comments/{id}.json{qs}");

	let data = match auth.bearer_token() {
		Some(_) => authed_json(path, false, &auth).await,
		None => anon_json(path, false).await,
	}
	.map_err(|e| e)?;

	json_response(200, data)
}

/// `GET /api/v1/me` — return the authenticated user's Reddit profile.
///
/// Requires authentication. Returns 401 if anonymous.
pub async fn me(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = api_auth(&req);

	if !auth.is_authenticated() {
		return json_error(401, "Authentication required — provide Authorization: Bearer <token> header or log in via /login");
	}

	let data = authed_json("/api/v1/me.json?raw_json=1".to_string(), false, &auth)
		.await
		.map_err(|e| e)?;

	json_response(200, data)
}

/// `POST /api/v1/vote` — vote on a post or comment.
///
/// JSON body: `{ "thing_id": "t3_abc123", "dir": 1 }`
/// Or form-encoded: `thing_id=t3_abc123&dir=1`
///
/// Requires authentication. No CSRF token required for API calls
/// (Bearer token acts as the auth proof).
pub async fn api_vote(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = api_auth(&req);

	if !auth.is_authenticated() {
		return json_error(401, "Authentication required");
	}

	// Read body (enforce size limit)
	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return json_error(413, "Request body too large");
	}

	// Try JSON first, then form-encoded
	let (thing_id, dir) = if let Ok(json_body) = serde_json::from_slice::<Value>(&body_bytes) {
		let thing_id = json_body["thing_id"].as_str().unwrap_or("").to_string();
		let dir = json_body["dir"].as_i64().unwrap_or(0) as i8;
		(thing_id, dir)
	} else {
		let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes)
			.map(|(k, v)| (k.into_owned(), v.into_owned()))
			.collect();
		let thing_id = form.get("thing_id").cloned().unwrap_or_default();
		let dir = form.get("dir").and_then(|s| s.parse::<i8>().ok()).unwrap_or(0);
		(thing_id, dir)
	};

	if thing_id.is_empty() {
		return json_error(400, "Missing 'thing_id'");
	}

	if ![-1i8, 0, 1].contains(&dir) {
		return json_error(400, "Invalid 'dir' — must be 1, 0, or -1");
	}

	let body_str = format!(
		"id={}&dir={}&rank=10",
		percent_encoding::utf8_percent_encode(&thing_id, percent_encoding::NON_ALPHANUMERIC),
		dir,
	);

	authed_post("/api/vote".to_string(), body_str, &auth)
		.await
		.map_err(|e| e)?;

	json_response(200, json!({ "ok": true }))
}
