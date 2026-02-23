//! Authentication subsystem for redlib-extended.
//!
//! Supports four auth modes, resolved in priority order per request:
//!   1. `REDLIB_RAW_TOKEN` — raw bearer token from environment (headless/script use)
//!   2. `rl_session` cookie — real Reddit OAuth session (user login flow)
//!   3. Anonymous — falls through to the spoofed anonymous `OAUTH_CLIENT`
//!
//! Session cookies are encrypted with AES-256-GCM. The key is derived via
//! HKDF-SHA256 from `REDLIB_SESSION_SECRET`. No server-side DB required.

use std::collections::HashMap;
use std::sync::LazyLock;

use aes_gcm::{
	aead::{Aead, KeyInit},
	Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use cookie::{Cookie, SameSite};
use hkdf::Hkdf;
use hyper::{Body, Method, Request, Response};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::config::CONFIG;
use crate::server::{RequestExt, ResponseExt};
use crate::utils::redirect;

/// Maximum POST body size accepted by auth handlers (4 KiB — login/logout forms are tiny).
const MAX_BODY_SIZE: usize = 4 * 1024;

/// Ephemeral 32-byte key used when `REDLIB_SESSION_SECRET` is not configured.
/// Sessions encrypted with this key do NOT survive server restarts.
static EPHEMERAL_SESSION_KEY: LazyLock<[u8; 32]> = LazyLock::new(|| {
	eprintln!(
		"WARNING [redlib-extended]: REDLIB_SESSION_SECRET is not set. \
		Using a random ephemeral key — all sessions will be invalidated on restart. \
		Set REDLIB_SESSION_SECRET to a 32+ byte random string for persistent sessions."
	);
	let mut key = [0u8; 32];
	rand::rngs::OsRng.fill_bytes(&mut key);
	key
});

// Cookie names
pub const SESSION_COOKIE: &str = "rl_session";
const CSRF_COOKIE: &str = "rl_csrf";

/// Reddit OAuth scopes requested during the user login flow.
const OAUTH_SCOPES: &str = "identity read vote subscribe history save";

// ----- Session data -----

/// Serializable session payload, stored AES-256-GCM encrypted in `rl_session` cookie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
	/// Reddit OAuth access token (short-lived, ~1 hour).
	pub access_token: String,
	/// Reddit OAuth refresh token (long-lived; used to get new access tokens).
	pub refresh_token: String,
	/// Authenticated Reddit username.
	pub username: String,
	/// Unix timestamp at which the access token expires.
	pub expires_at: i64,
	/// Per-session CSRF token embedded in HTML forms to prevent CSRF attacks.
	pub csrf_token: String,
}

// ----- Auth context -----

/// Resolved per-request authentication context.
///
/// Created by `AuthContext::from_request()` at the top of any handler that
/// needs to know who the caller is.
#[derive(Debug, Clone)]
pub enum AuthContext {
	/// A logged-in Reddit user with a real OAuth access token.
	UserSession(SessionData),
	/// A raw bearer token provided via `REDLIB_RAW_TOKEN` env var.
	RawBearer(String),
	/// Anonymous — no credentials; falls through to the spoofed `OAUTH_CLIENT`.
	Anonymous,
}

impl AuthContext {
	/// Resolve auth context for a request. Resolution order:
	/// 1. `REDLIB_RAW_TOKEN` — direct bearer token (highest priority)
	/// 2. `REDLIB_BROWSER_TOKEN` — browser-exported `token_v2` JWT, decoded to bearer
	/// 3. `rl_session` cookie — encrypted OAuth user session
	/// 4. `Anonymous`
	pub fn from_request(req: &Request<Body>) -> Self {
		// Priority 1: raw bearer token from config
		if let Some(token) = CONFIG.raw_token.clone().filter(|s| !s.is_empty()) {
			return AuthContext::RawBearer(token);
		}

		// Priority 2: browser-exported token (token_v2 JWT or raw bearer)
		if let Some(raw) = CONFIG.browser_token.clone().filter(|s| !s.is_empty()) {
			let bearer = decode_browser_token(&raw).unwrap_or(raw);
			return AuthContext::RawBearer(bearer);
		}

		// Priority 3: encrypted session cookie
		if let Some(cookie) = req.cookie(SESSION_COOKIE) {
			if let Some(session) = decrypt_session(cookie.value()) {
				// Accept tokens with up to 30s of grace past expiry (clock skew / slow requests)
				let now = OffsetDateTime::now_utc().unix_timestamp();
				if session.expires_at > now - 30 {
					return AuthContext::UserSession(session);
				}
			}
		}

		// Priority 4: anonymous
		AuthContext::Anonymous
	}

	/// Return the bearer token for authenticated Reddit API calls, if any.
	pub fn bearer_token(&self) -> Option<&str> {
		match self {
			AuthContext::UserSession(s) => Some(&s.access_token),
			AuthContext::RawBearer(t) => Some(t.as_str()),
			AuthContext::Anonymous => None,
		}
	}

	/// Return the logged-in username, if any.
	pub fn username(&self) -> Option<&str> {
		match self {
			AuthContext::UserSession(s) => Some(s.username.as_str()),
			_ => None,
		}
	}

	/// Return the CSRF token for form embedding. Empty string when anonymous.
	pub fn csrf_token(&self) -> String {
		match self {
			AuthContext::UserSession(s) => s.csrf_token.clone(),
			_ => String::new(),
		}
	}

	/// Whether there is an active authenticated session (user or raw token).
	pub fn is_authenticated(&self) -> bool {
		!matches!(self, AuthContext::Anonymous)
	}
}

// ----- Session encryption / decryption -----

/// Derive a 32-byte AES-256-GCM key from `REDLIB_SESSION_SECRET` via HKDF-SHA256.
///
/// If `REDLIB_SESSION_SECRET` is not set, falls back to a per-process ephemeral
/// random key (sessions will not survive restarts — a warning is printed once).
fn session_key() -> [u8; 32] {
	match CONFIG.session_secret.as_deref().filter(|s| !s.is_empty()) {
		Some(secret) => {
			let hk = Hkdf::<Sha256>::new(None, secret.as_bytes());
			let mut key = [0u8; 32];
			hk.expand(b"redlib-session-v1", &mut key).expect("HKDF expand failed");
			key
		}
		None => *EPHEMERAL_SESSION_KEY,
	}
}

/// Attempt to decode a browser-exported Reddit `token_v2` JWT and extract
/// the bearer token from the payload.
///
/// Reddit's `token_v2` cookie is a JWT whose payload contains an `access_token`
/// field (or `token` in some client builds). If the input is not a valid JWT,
/// it is returned as-is so callers can treat it as a raw bearer token.
pub fn decode_browser_token(raw: &str) -> Option<String> {
	// JWT format: header.payload.signature — we only need the payload
	let parts: Vec<&str> = raw.splitn(3, '.').collect();
	if parts.len() < 2 {
		return None;
	}
	// Base64url-decode the payload (no padding required by base64url)
	let payload_bytes = general_purpose::URL_SAFE_NO_PAD.decode(parts[1]).ok()?;
	let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;

	// Try common field names used by Reddit's various client builds
	for field in &["access_token", "token", "accessToken"] {
		if let Some(tok) = payload[field].as_str().filter(|s| !s.is_empty()) {
			return Some(tok.to_string());
		}
	}
	None
}

/// Encrypt `SessionData` to a base64 string suitable for a cookie value.
///
/// Wire format: `base64(nonce_12_bytes || aes_gcm_ciphertext)`
pub fn encrypt_session(data: &SessionData) -> Option<String> {
	let key_bytes = session_key();
	let cipher = Aes256Gcm::new_from_slice(&key_bytes).ok()?;

	let mut nonce_bytes = [0u8; 12];
	rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
	let nonce = Nonce::from_slice(&nonce_bytes);

	let plaintext = serde_json::to_vec(data).ok()?;
	let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).ok()?;

	let mut combined = nonce_bytes.to_vec();
	combined.extend_from_slice(&ciphertext);

	Some(general_purpose::STANDARD.encode(combined))
}

/// Decrypt and deserialize a base64 session cookie value.
/// Returns `None` on any decryption or parse failure (invalid key, tampered data, etc.)
pub fn decrypt_session(encoded: &str) -> Option<SessionData> {
	let combined = general_purpose::STANDARD.decode(encoded).ok()?;
	if combined.len() < 12 {
		return None;
	}
	let (nonce_bytes, ciphertext) = combined.split_at(12);
	let key_bytes = session_key();
	let cipher = Aes256Gcm::new_from_slice(&key_bytes).ok()?;
	let nonce = Nonce::from_slice(nonce_bytes);
	let plaintext = cipher.decrypt(nonce, ciphertext).ok()?;
	serde_json::from_slice(&plaintext).ok()
}

/// Returns `true` when the `Secure` cookie attribute should be set.
///
/// Enable via `REDLIB_SECURE_COOKIES=on` (recommended for HTTPS deployments).
/// Defaults to `false` to avoid breaking plain-HTTP local setups.
fn secure_cookies() -> bool {
	CONFIG.secure_cookies.as_deref().map(|v| v.eq_ignore_ascii_case("on") || v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false)
}

/// Reserialize a mutated `SessionData` back into the session cookie on a response.
pub fn update_session_cookie(response: &mut Response<Body>, data: &SessionData) {
	if let Some(encrypted) = encrypt_session(data) {
		response.insert_cookie(
			Cookie::build((SESSION_COOKIE.to_string(), encrypted))
				.path("/")
				.http_only(true)
				.secure(secure_cookies())
				.same_site(SameSite::Lax)
				.expires(OffsetDateTime::now_utc() + Duration::weeks(4))
				.into(),
		);
	}
}

// ----- CSRF helpers -----

/// Validate the CSRF token submitted in a POST form body against the session.
/// For `Anonymous` / `RawBearer`, CSRF is not applicable — returns `Ok(())`.
pub fn validate_csrf_token(auth: &AuthContext, submitted: &str) -> Result<(), String> {
	if let AuthContext::UserSession(session) = auth {
		if submitted != session.csrf_token {
			return Err("CSRF token mismatch — request rejected".to_string());
		}
	}
	Ok(())
}

// ----- OAuth routes -----

/// `GET /login` — generate a CSRF state token, then redirect to Reddit's
/// OAuth consent page.
pub async fn login_redirect(_req: Request<Body>) -> Result<Response<Body>, String> {
	let client_id = CONFIG.oauth_client_id.clone().ok_or("REDLIB_OAUTH_CLIENT_ID is not configured")?;
	let redirect_uri = CONFIG.oauth_redirect_uri.clone().ok_or("REDLIB_OAUTH_REDIRECT_URI is not configured")?;

	// CSRF state token — stored in a short-lived cookie, compared on callback
	let state = Uuid::new_v4().to_string();

	let authorize_url = format!(
		"https://www.reddit.com/api/v1/authorize?client_id={client_id}&response_type=code&state={state}&redirect_uri={redirect_uri}&duration=permanent&scope={scopes}",
		client_id = percent_encoding::utf8_percent_encode(&client_id, percent_encoding::NON_ALPHANUMERIC),
		state = &state,
		redirect_uri = percent_encoding::utf8_percent_encode(&redirect_uri, percent_encoding::NON_ALPHANUMERIC),
		scopes = OAUTH_SCOPES,
	);

	let mut response = redirect(&authorize_url);
	response.insert_cookie(
		Cookie::build((CSRF_COOKIE.to_string(), state))
			.path("/")
			.http_only(true)
			.secure(secure_cookies())
			.same_site(SameSite::Lax)
			// Expire the CSRF cookie after 10 minutes — enough time to complete login
			.expires(OffsetDateTime::now_utc() + Duration::minutes(10))
			.into(),
	);
	Ok(response)
}

/// `GET /auth/callback` — handle Reddit's OAuth redirect, exchange the code
/// for tokens, and set the encrypted session cookie.
pub async fn oauth_callback(req: Request<Body>) -> Result<Response<Body>, String> {
	let client_id = CONFIG.oauth_client_id.clone().ok_or("REDLIB_OAUTH_CLIENT_ID is not configured")?;
	let client_secret = CONFIG.oauth_client_secret.clone().ok_or("REDLIB_OAUTH_CLIENT_SECRET is not configured")?;
	let redirect_uri = CONFIG.oauth_redirect_uri.clone().ok_or("REDLIB_OAUTH_REDIRECT_URI is not configured")?;

	// Parse query parameters from the callback URL
	let query = req.uri().query().unwrap_or("");
	let params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
		.map(|(k, v)| (k.into_owned(), v.into_owned()))
		.collect();

	// Validate CSRF state
	let state = params.get("state").ok_or("Missing 'state' parameter in callback")?;
	let csrf_cookie = req.cookie(CSRF_COOKIE).ok_or("Missing CSRF cookie — possible CSRF attack or cookie expired")?;
	if state != csrf_cookie.value() {
		return Err("CSRF state mismatch — possible CSRF attack".to_string());
	}

	// Check for OAuth error from Reddit
	if let Some(err) = params.get("error") {
		return Err(format!("Reddit OAuth error: {err}"));
	}

	let code = params.get("code").ok_or("Missing 'code' parameter in callback")?;

	// Exchange authorization code for access + refresh tokens
	let tokens = exchange_code(&client_id, &client_secret, code, &redirect_uri).await?;

	// Fetch the username from Reddit's /api/v1/me
	let username = fetch_username(&tokens.access_token).await.unwrap_or_else(|_| "unknown".to_string());

	let expires_at = OffsetDateTime::now_utc().unix_timestamp() + tokens.expires_in as i64;

	let session = SessionData {
		access_token: tokens.access_token,
		refresh_token: tokens.refresh_token,
		username,
		expires_at,
		csrf_token: Uuid::new_v4().to_string(),
	};

	let encrypted = encrypt_session(&session).ok_or("Failed to encrypt session data")?;

	let mut response = redirect("/");
	response.insert_cookie(
		Cookie::build((SESSION_COOKIE.to_string(), encrypted))
			.path("/")
			.http_only(true)
			.secure(secure_cookies())
			.same_site(SameSite::Lax)
			.expires(OffsetDateTime::now_utc() + Duration::weeks(4))
			.into(),
	);
	// Clear the CSRF cookie — it's served its purpose
	response.remove_cookie(CSRF_COOKIE.to_string());

	Ok(response)
}

/// `POST /logout` — validate CSRF token, clear the session cookie.
pub async fn logout(req: Request<Body>) -> Result<Response<Body>, String> {
	// Extract auth context before consuming the body
	let auth = AuthContext::from_request(&req);

	// Read and parse POST body for CSRF token (with size limit)
	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes)
		.map(|(k, v)| (k.into_owned(), v.into_owned()))
		.collect();

	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
	validate_csrf_token(&auth, submitted_csrf)?;

	let mut response = redirect("/");
	response.remove_cookie(SESSION_COOKIE.to_string());
	Ok(response)
}

// ----- Reddit API helpers -----

#[derive(Debug, Deserialize)]
struct TokenResponse {
	access_token: String,
	refresh_token: String,
	expires_in: u64,
}

/// Exchange an authorization code for a Reddit access + refresh token pair.
async fn exchange_code(client_id: &str, client_secret: &str, code: &str, redirect_uri: &str) -> Result<TokenResponse, String> {
	let credentials = general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
	let body_str = format!(
		"grant_type=authorization_code&code={}&redirect_uri={}",
		percent_encoding::utf8_percent_encode(code, percent_encoding::NON_ALPHANUMERIC),
		percent_encoding::utf8_percent_encode(redirect_uri, percent_encoding::NON_ALPHANUMERIC),
	);

	let request = Request::builder()
		.method(Method::POST)
		.uri("https://www.reddit.com/api/v1/access_token")
		.header("Authorization", format!("Basic {credentials}"))
		.header("Content-Type", "application/x-www-form-urlencoded")
		.header("User-Agent", crate::config::get_user_agent())
		.body(Body::from(body_str))
		.map_err(|e| e.to_string())?;

	let client = &crate::client::CLIENT;
	let resp = client.request(request).await.map_err(|e| e.to_string())?;

	if !resp.status().is_success() {
		let status = resp.status();
		let bytes = hyper::body::to_bytes(resp.into_body()).await.unwrap_or_default();
		// Log the full Reddit error server-side, return a generic message to the client
		// to avoid leaking OAuth details (client ID, scope info, etc.) in error responses.
		log::error!("OAuth token exchange failed: HTTP {status} — {}", String::from_utf8_lossy(&bytes));
		return Err("Authentication failed — could not obtain access token from Reddit".to_string());
	}

	let bytes = hyper::body::to_bytes(resp.into_body()).await.map_err(|e| e.to_string())?;
	serde_json::from_slice::<TokenResponse>(&bytes).map_err(|_| {
		log::error!("OAuth token exchange: failed to parse Reddit token response");
		"Authentication failed — unexpected response from Reddit".to_string()
	})
}

/// Fetch the authenticated user's Reddit username via `/api/v1/me`.
async fn fetch_username(access_token: &str) -> Result<String, String> {
	let request = Request::builder()
		.method(Method::GET)
		.uri("https://oauth.reddit.com/api/v1/me")
		.header("Authorization", format!("Bearer {access_token}"))
		.header("User-Agent", crate::config::get_user_agent())
		.body(Body::empty())
		.map_err(|e| e.to_string())?;

	let client = &crate::client::CLIENT;
	let resp = client.request(request).await.map_err(|e| e.to_string())?;
	let bytes = hyper::body::to_bytes(resp.into_body()).await.map_err(|e| e.to_string())?;
	let json: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;

	json["name"]
		.as_str()
		.map(|s| s.to_string())
		.ok_or_else(|| "Could not read 'name' field from /api/v1/me response".to_string())
}

/// Refresh an expired access token using the stored refresh token.
/// Returns `(new_access_token, new_expires_at_unix_timestamp)`.
pub async fn refresh_access_token(refresh_token: &str) -> Result<(String, i64), String> {
	let client_id = CONFIG.oauth_client_id.clone().ok_or("REDLIB_OAUTH_CLIENT_ID not configured")?;
	let client_secret = CONFIG.oauth_client_secret.clone().ok_or("REDLIB_OAUTH_CLIENT_SECRET not configured")?;

	let credentials = general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
	let body_str = format!(
		"grant_type=refresh_token&refresh_token={}",
		percent_encoding::utf8_percent_encode(refresh_token, percent_encoding::NON_ALPHANUMERIC),
	);

	let request = Request::builder()
		.method(Method::POST)
		.uri("https://www.reddit.com/api/v1/access_token")
		.header("Authorization", format!("Basic {credentials}"))
		.header("Content-Type", "application/x-www-form-urlencoded")
		.header("User-Agent", crate::config::get_user_agent())
		.body(Body::from(body_str))
		.map_err(|e| e.to_string())?;

	let client = &crate::client::CLIENT;
	let resp = client.request(request).await.map_err(|e| e.to_string())?;
	let bytes = hyper::body::to_bytes(resp.into_body()).await.map_err(|e| e.to_string())?;
	let json: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;

	let new_token = json["access_token"]
		.as_str()
		.ok_or("Missing access_token in refresh response")?
		.to_string();
	let expires_in = json["expires_in"].as_i64().unwrap_or(3600);
	let expires_at = OffsetDateTime::now_utc().unix_timestamp() + expires_in;

	Ok((new_token, expires_at))
}
