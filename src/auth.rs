//! Authentication subsystem for redlib-extended.
//!
//! Supports four auth modes, resolved in priority order per request:
//!   1. `REDLIB_RAW_TOKEN` — raw bearer token from environment (headless/script use)
//!   2. `rl_session` cookie — real Reddit OAuth session (user login flow)
//!   3. Anonymous — falls through to the spoofed anonymous `OAUTH_CLIENT`
//!
//! Session cookies are encrypted with AES-256-GCM. The key is derived via
//! HKDF-SHA256 from `REDLIB_SESSION_SECRET`. No server-side DB required.

#![allow(clippy::cmp_owned)]

use std::collections::HashMap;
use std::sync::LazyLock;

use askama::Template;

use aes_gcm::{
	aead::{Aead, KeyInit},
	Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use cookie::{Cookie, SameSite};
use hkdf::Hkdf;
use hyper::{Body, Method, Request, Response};
use rand::RngCore;
use serde::Deserialize;
use serde::Serialize;
use sha2::Sha256;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::client;
use crate::config::{set_runtime_user_agent, CONFIG};
use crate::server::{RequestExt, ResponseExt};
use crate::token_import;
use crate::utils::{redirect, template, Preferences};

// ----- Login page template -----

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage<'a> {
	prefs: Preferences,
	url: String,
	error: Option<&'a str>,
	ssh_host: String,
	ssh_user: String,
	local_profiles: Vec<LoginLocalProfile>,
}

#[derive(Debug, Clone)]
struct LoginLocalProfile {
	id: String,
	label: String,
	browser: String,
}

/// Maximum POST body size accepted by auth handlers (16 KiB — allows pasted SSH private key).
const MAX_BODY_SIZE: usize = 16 * 1024;

/// Holds path to a temporary private key file; removes it on drop.
struct TempKeyFile(std::path::PathBuf);
impl Drop for TempKeyFile {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.0);
	}
}

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

// Cookie names - use __Host- prefix for enhanced security when HTTPS is enabled
pub fn session_cookie_name() -> &'static str {
	if secure_cookies() {
		"__Host-rl_session"
	} else {
		"rl_session"
	}
}

/// Cookie to track which account is active (stores username).
pub fn active_session_cookie_name() -> &'static str {
	if secure_cookies() {
		"__Host-rl_active"
	} else {
		"rl_active"
	}
}

pub fn csrf_cookie_name() -> &'static str {
	if secure_cookies() {
		"__Host-rl_csrf"
	} else {
		"rl_csrf"
	}
}

pub fn subscriptions_cookie_name() -> &'static str {
	if secure_cookies() {
		"__Host-reddit_subscriptions"
	} else {
		"reddit_subscriptions"
	}
}

/// Reddit OAuth scopes requested during the user login flow.
const OAUTH_SCOPES: &str = "identity read vote subscribe history save submit privatemessages";

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

/// Serializable session vault — holds multiple user sessions for account switching.
/// Stored AES-256-GCM encrypted in `rl_session` cookie.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionVault {
	/// List of session data for all accounts.
	pub sessions: Vec<SessionData>,
}

impl SessionVault {
	pub fn new() -> Self {
		Self { sessions: Vec::new() }
	}

	pub fn add(&mut self, session: SessionData) {
		// Remove any existing session for the same username
		self.sessions.retain(|s| s.username != session.username);
		self.sessions.push(session);
	}

	pub fn remove(&mut self, username: &str) {
		self.sessions.retain(|s| s.username != username);
	}

	pub fn get(&self, username: &str) -> Option<&SessionData> {
		self.sessions.iter().find(|s| s.username == username)
	}

	pub fn active_session(&self) -> Option<&SessionData> {
		self.sessions.first()
	}

	pub fn usenames(&self) -> Vec<String> {
		self.sessions.iter().map(|s| s.username.clone()).collect()
	}
}

impl Default for SessionVault {
	fn default() -> Self {
		Self::new()
	}
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
	/// 3. `rl_session` cookie — encrypted OAuth session vault
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

		// Priority 3: encrypted session vault cookie
		if let Some(cookie) = req.cookie(session_cookie_name()) {
			if let Some(vault) = decrypt_vault(cookie.value()) {
				// Find active session: first by rl_active cookie, then first in vault
				let active_username = req.cookie(active_session_cookie_name()).map(|c| c.value().to_string());
				let now = OffsetDateTime::now_utc().unix_timestamp();

				// Try active username first
				if let Some(username) = active_username {
					if let Some(session) = vault.get(&username) {
						if session.expires_at > now - 30 {
							return AuthContext::UserSession(session.clone());
						}
					}
				}

				// Fall back to first valid session
				for session in &vault.sessions {
					if session.expires_at > now - 30 {
						return AuthContext::UserSession(session.clone());
					}
				}
			}
		}

		// Priority 4: anonymous
		AuthContext::Anonymous
	}

	/// Return all usernames in the session vault (for account switching UI).
	pub fn all_usernames(req: &Request<Body>) -> Vec<String> {
		if let Some(cookie) = req.cookie(session_cookie_name()) {
			if let Some(vault) = decrypt_vault(cookie.value()) {
				return vault.usenames();
			}
		}
		Vec::new()
	}

	/// Return the active username from the cookie (for display).
	pub fn active_username(req: &Request<Body>) -> Option<String> {
		if let Some(cookie) = req.cookie(session_cookie_name()) {
			if let Some(vault) = decrypt_vault(cookie.value()) {
				// Check active cookie first
				if let Some(active) = req.cookie(active_session_cookie_name()) {
					let active_str = active.value();
					if vault.get(active_str).is_some() {
						return Some(active_str.to_string());
					}
				}
				// Default to first session
				return vault.active_session().map(|s| s.username.clone());
			}
		}
		None
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

	/// Return a reference to the session data when the context is a user session.
	/// Used by the client layer to refresh the access token on 401 and return updated session to set cookie.
	pub fn session_data(&self) -> Option<&SessionData> {
		match self {
			AuthContext::UserSession(s) => Some(s),
			_ => None,
		}
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

/// Encrypt `SessionVault` to a base64 string suitable for a cookie value.
pub fn encrypt_vault(vault: &SessionVault) -> Option<String> {
	let key_bytes = session_key();
	let cipher = Aes256Gcm::new_from_slice(&key_bytes).ok()?;

	let mut nonce_bytes = [0u8; 12];
	rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
	let nonce = Nonce::from_slice(&nonce_bytes);

	let plaintext = serde_json::to_vec(vault).ok()?;
	let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).ok()?;

	let mut combined = nonce_bytes.to_vec();
	combined.extend_from_slice(&ciphertext);

	Some(general_purpose::STANDARD.encode(combined))
}

/// Decrypt and deserialize a base64 session vault cookie value.
pub fn decrypt_vault(encoded: &str) -> Option<SessionVault> {
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

const AUTH_LANDING_PATH: &str = "/settings";

/// Add a new session to the vault and return a response with updated cookies.
/// Sets the new session as active.
pub fn add_session_to_vault(session: SessionData) -> Result<Response<Body>, String> {
	let mut vault = SessionVault::new();
	vault.add(session.clone());

	let encrypted = encrypt_vault(&vault).ok_or("Failed to encrypt session vault")?;
	let mut response = redirect(AUTH_LANDING_PATH);
	response.insert_cookie(
		Cookie::build((session_cookie_name(), encrypted))
			.path("/")
			.http_only(true)
			.secure(secure_cookies())
			.same_site(SameSite::Lax)
			.expires(OffsetDateTime::now_utc() + Duration::weeks(4))
			.into(),
	);
	// Set active session to the new username
	response.insert_cookie(
		Cookie::build((active_session_cookie_name(), session.username.clone()))
			.path("/")
			.http_only(true)
			.secure(secure_cookies())
			.same_site(SameSite::Lax)
			.expires(OffsetDateTime::now_utc() + Duration::weeks(4))
			.into(),
	);
	Ok(response)
}

/// Switch to a different account in the vault.
pub fn switch_active_session(username: &str, vault_cookie: Option<&str>) -> Result<Response<Body>, String> {
	// Verify the username exists in the vault
	if let Some(cookie_val) = vault_cookie {
		let vault = decrypt_vault(cookie_val).ok_or("No sessions found")?;
		if vault.get(username).is_none() {
			return Err("Session not found".to_string());
		}
	}

	let mut response = redirect(AUTH_LANDING_PATH);
	response.insert_cookie(
		Cookie::build((active_session_cookie_name(), username.to_string()))
			.path("/")
			.http_only(true)
			.secure(secure_cookies())
			.same_site(SameSite::Lax)
			.expires(OffsetDateTime::now_utc() + Duration::weeks(4))
			.into(),
	);
	Ok(response)
}

/// Remove a session from the vault.
pub fn remove_session_from_vault(username: &str, vault_cookie: Option<&str>) -> Result<Response<Body>, String> {
	let mut vault = vault_cookie.and_then(|c| decrypt_vault(c)).unwrap_or_default();

	// If removing active session, switch to another if available
	let had_active = vault.get(username).is_some();
	let was_only = vault.sessions.len() == 1;
	vault.remove(username);

	let encrypted = encrypt_vault(&vault).ok_or("Failed to encrypt session vault")?;
	let mut response = redirect(AUTH_LANDING_PATH);
	response.insert_cookie(
		Cookie::build((session_cookie_name(), encrypted))
			.path("/")
			.http_only(true)
			.secure(secure_cookies())
			.same_site(SameSite::Lax)
			.expires(OffsetDateTime::now_utc() + Duration::weeks(4))
			.into(),
	);

	// If we removed the active session and there are others, switch to first
	if had_active && !was_only {
		if let Some(first) = vault.active_session() {
			response.insert_cookie(
				Cookie::build((active_session_cookie_name(), first.username.clone()))
					.path("/")
					.http_only(true)
					.secure(secure_cookies())
					.same_site(SameSite::Lax)
					.expires(OffsetDateTime::now_utc() + Duration::weeks(4))
					.into(),
			);
		}
	} else if was_only || vault.sessions.is_empty() {
		// No sessions left - clear active cookie
		response.remove_cookie(active_session_cookie_name().to_string());
	}
	Ok(response)
}

/// Returns `true` when the `Secure` cookie attribute should be set.
///
/// Enable via `REDLIB_SECURE_COOKIES=on` (recommended for HTTPS deployments).
/// Defaults to `false` to avoid breaking plain-HTTP local setups.
pub fn secure_cookies() -> bool {
	CONFIG
		.secure_cookies
		.as_deref()
		.map(|v| v.eq_ignore_ascii_case("on") || v == "1" || v.eq_ignore_ascii_case("true"))
		.unwrap_or(false)
}

/// Returns the SSH connection timeout in seconds. Default: 15.
fn ssh_timeout() -> u64 {
	CONFIG.ssh_timeout.as_deref().and_then(|v| v.parse().ok()).unwrap_or(15)
}

/// Returns whether strict SSH host key checking is enabled.
/// When disabled (default), new hosts are automatically added to known_hosts.
/// When enabled, the host must already be in known_hosts.
fn ssh_strict_host_key_checking() -> bool {
	CONFIG
		.ssh_strict_host_key_checking
		.as_deref()
		.map(|v| v.eq_ignore_ascii_case("on") || v == "1" || v.eq_ignore_ascii_case("true"))
		.unwrap_or(false)
}

/// Reserialize a mutated `SessionData` back into the session cookie on a response.
pub fn update_session_cookie(response: &mut Response<Body>, data: &SessionData) {
	if let Some(encrypted) = encrypt_session(data) {
		response.insert_cookie(
			Cookie::build((session_cookie_name(), encrypted))
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

/// `GET /login` — show the login choice page (Reddit OAuth or SSH import).
/// Redirects to `/settings` if already authenticated.
pub async fn login_page(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);
	if auth.is_authenticated() {
		return Ok(redirect(AUTH_LANDING_PATH));
	}
	let prefs = Preferences::new(&req);
	let page = build_login_page(&prefs, None);
	Ok(template(&page))
}

fn build_login_page<'a>(prefs: &Preferences, error: Option<&'a str>) -> LoginPage<'a> {
	let local_profiles = token_import::discover_local_profiles()
		.into_iter()
		.map(|p| LoginLocalProfile {
			id: p.id,
			label: p.label,
			browser: p.browser,
		})
		.collect();

	LoginPage {
		url: "/login".to_string(),
		ssh_host: CONFIG.ssh_host.clone().unwrap_or_else(|| "kspld0".to_string()),
		ssh_user: CONFIG.ssh_user.clone().unwrap_or_else(|| "keith".to_string()),
		error,
		prefs: prefs.clone(),
		local_profiles,
	}
}

async fn complete_browser_import_login(bearer_token: String, user_agent: Option<String>) -> Result<Response<Body>, String> {
	let username = fetch_username(&bearer_token).await.unwrap_or_else(|_| "unknown".to_string());
	let reddit_subs = client::fetch_subscribed_subreddits_with_bearer(&bearer_token).await.unwrap_or_default();

	if let Some(ua) = user_agent.filter(|s| !s.is_empty()) {
		set_runtime_user_agent(ua);
	}

	let session = SessionData {
		access_token: bearer_token,
		refresh_token: String::new(),
		username,
		expires_at: OffsetDateTime::now_utc().unix_timestamp() + 6 * 3600,
		csrf_token: Uuid::new_v4().to_string(),
	};

	let mut response = add_session_to_vault(session)?;

	if !reddit_subs.is_empty() {
		response.insert_cookie(
			Cookie::build((subscriptions_cookie_name(), reddit_subs.join("+")))
				.path("/")
				.http_only(true)
				.secure(secure_cookies())
				.same_site(SameSite::Lax)
				.expires(OffsetDateTime::now_utc() + Duration::weeks(4))
				.into(),
		);
	}

	Ok(response)
}

/// `POST /login/reddit` — generate a CSRF state token, then redirect to Reddit's
/// OAuth consent page.
pub async fn login_reddit(req: Request<Body>) -> Result<Response<Body>, String> {
	let client_id = match CONFIG.oauth_client_id.as_deref() {
		Some(id) if !id.trim().is_empty() && !id.eq_ignore_ascii_case("placeholder") => id.to_string(),
		_ => {
			let prefs = Preferences::new(&req);
			return render_login_error(
				&prefs,
				"Reddit OAuth is not configured. Set REDLIB_OAUTH_CLIENT_ID and REDLIB_OAUTH_CLIENT_SECRET in redlibe-secrets. In your Reddit app (reddit.com/prefs/apps) set redirect URI to https://redlibe.home/auth/callback.",
			);
		}
	};
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
		Cookie::build((csrf_cookie_name(), state))
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

/// `POST /login/ssh-import` — extract a Reddit bearer token from a Firefox or
/// LibreWolf installation on a remote machine via SSH and sqlite3.
///
/// Form fields: `ssh_host`, `ssh_user`, `browser` (librewolf | firefox).
/// Reads cookies.sqlite on the remote, decodes the token_v2 JWT, detects the
/// browser version/arch for a matching User-Agent, then creates a session.
pub async fn login_ssh_import(req: Request<Body>) -> Result<Response<Body>, String> {
	let prefs = Preferences::new(&req);

	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();

	let ssh_host = form
		.get("ssh_host")
		.map(|s| s.trim().to_string())
		.unwrap_or_else(|| CONFIG.ssh_host.clone().unwrap_or_else(|| "kspld0".to_string()));
	let ssh_user = form
		.get("ssh_user")
		.map(|s| s.trim().to_string())
		.unwrap_or_else(|| CONFIG.ssh_user.clone().unwrap_or_else(|| "keith".to_string()));
	let browser = form.get("browser").map(|s| s.as_str()).unwrap_or("auto");

	// Validate ssh_host and ssh_user to prevent injection (used as CLI args, not shell strings)
	if !ssh_host.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == '_') {
		return render_login_error(&prefs, "Invalid SSH host — only alphanumeric, dots, hyphens, underscores allowed");
	}
	if !ssh_user.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
		return render_login_error(&prefs, "Invalid SSH user — only alphanumeric, underscores, hyphens allowed");
	}

	let has_pasted_key = form.get("ssh_private_key").map(|s| !s.trim().is_empty()).unwrap_or(false);
	let ssh_password = form.get("ssh_password").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
	let has_config_key = CONFIG.ssh_key.as_ref().map(|k| !k.trim().is_empty()).unwrap_or(false);
	if !has_pasted_key && ssh_password.is_none() && !has_config_key {
		return render_login_error(&prefs, "Provide either an SSH private key or an SSH password (or both).");
	}

	// Use pasted private key from form if provided; otherwise we'll use password or config key
	let (key_path_opt, _temp_guard) = if let Some(pasted) = form.get("ssh_private_key").map(|s| s.trim()) {
		if pasted.is_empty() {
			(None, None)
		} else {
			// Normalize for OpenSSH: strip BOM, CRLF -> LF, ensure trailing newline (avoids "error in libcrypto")
			let normalized = pasted.strip_prefix('\u{feff}').unwrap_or(pasted).replace("\r\n", "\n").replace('\r', "\n");
			let normalized = if normalized.ends_with('\n') { normalized } else { format!("{normalized}\n") };
			// Detect public key paste (one line starting with "ssh-rsa " or "ssh-ed25519 ") — SSH needs the private key to connect
			if normalized.starts_with("ssh-rsa ") || normalized.starts_with("ssh-ed25519 ") {
				if normalized.lines().count() <= 1 {
					return render_login_error(
						&prefs,
						"You pasted a public key (the one-line .pub file). SSH login requires your private key: open the file without .pub (e.g. ~/.ssh/id_ed25519) and paste its full contents (starts with -----BEGIN ... PRIVATE KEY-----).",
					);
				}
			}
			let temp_dir = std::env::temp_dir();
			let path = temp_dir.join(format!("redlib_ssh_{}.key", Uuid::new_v4()));
			if std::fs::write(&path, normalized).is_err() {
				return render_login_error(&prefs, "Could not write temporary key file");
			}
			#[cfg(unix)]
			{
				use std::os::unix::fs::PermissionsExt;
				if std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).is_err() {
					let _ = std::fs::remove_file(&path);
					return render_login_error(&prefs, "Could not set key file permissions");
				}
			}
			let path_str = path.to_string_lossy().into_owned();
			(Some(path_str), Some(TempKeyFile(path)))
		}
	} else {
		(None, None)
	};

	// If no pasted key, use config key path only when password not provided (so key-only from server config)
	let key_path_opt = if key_path_opt.is_some() {
		key_path_opt
	} else if ssh_password.is_none() {
		let k = CONFIG.ssh_key.clone().unwrap_or_else(|| "~/.ssh/id_ed25519".to_string());
		Some(shellexpand::tilde(&k).into_owned())
	} else {
		None
	};

	// Run the extraction: key+passphrase when both provided, else key-only, else password-only
	let result = match (key_path_opt.as_ref(), ssh_password.as_ref()) {
		(Some(kp), Some(pass)) => {
			log::info!("SSH import: using key auth with passphrase");
			ssh_extract_token_key_passphrase(&ssh_host, &ssh_user, kp, pass, browser).await
		}
		(Some(kp), None) => {
			log::info!("SSH import: using key-only auth");
			ssh_extract_token(&ssh_host, &ssh_user, kp, browser).await
		}
		(None, Some(pass)) => {
			log::info!("SSH import: using password auth");
			ssh_extract_token_with_password(&ssh_host, &ssh_user, pass, browser).await
		}
		(None, None) => {
			return render_login_error(&prefs, "Provide either an SSH private key or an SSH password (or both).");
		}
	};

	match result {
		Err(e) => {
			let msg = if e.contains("Load key") && (e.contains("libcrypto") || e.contains("Permission denied (publickey)")) {
				format!(
					"SSH extraction failed: {e} \
					— Use your private key (starts with -----BEGIN ... PRIVATE KEY-----), not the public key. \
					If the key is passphrase-protected, enter the passphrase in the password field.",
				)
			} else {
				format!("SSH extraction failed: {e}")
			};
			render_login_error(&prefs, &msg)
		}
		Ok((bearer_token, user_agent)) => complete_browser_import_login(bearer_token, Some(user_agent)).await,
	}
}

/// `POST /login/local-import` — import a browser token from a local browser profile.
///
/// Form fields:
/// - `browser` (librewolf | firefox | chrome | edge)
/// - `profile_id` (optional discovered profile id)
/// - `profile_path` (optional manual override path)
pub async fn login_local_import(req: Request<Body>) -> Result<Response<Body>, String> {
	let prefs = Preferences::new(&req);
	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();

	let browser = form.get("browser").map(|s| s.as_str()).unwrap_or("firefox");
	let profile_id = form.get("profile_id").map(|s| s.as_str());
	let profile_path = form.get("profile_path").map(|s| s.as_str());

	match token_import::import_local(browser, profile_id, profile_path) {
		Ok(imported) => complete_browser_import_login(imported.bearer_token, imported.user_agent).await,
		Err(e) => {
			let msg = format!("Local import failed: {e}");
			render_login_error(&prefs, &msg)
		}
	}
}

/// Re-render the login page with an inline error message.
fn render_login_error(prefs: &Preferences, msg: &str) -> Result<Response<Body>, String> {
	let page = build_login_page(prefs, Some(msg));
	Ok(template(&page))
}

/// Returns `(find_dirs, version_bin)` for the remote shell script based on
/// the browser choice. "auto" searches all known Firefox-based profile dirs.
fn browser_script_parts(browser: &str) -> (&'static str, &'static str) {
	match browser {
		"firefox" => ("~/.mozilla/firefox ~/.var/app/org.mozilla.firefox/.mozilla/firefox", "firefox"),
		"librewolf" => (
			// LibreWolf may be at ~/.librewolf (traditional) or ~/.config/librewolf/librewolf (XDG/packaged)
			"~/.librewolf ~/.config/librewolf/librewolf ~/.var/app/io.gitlab.librewolf-community/.librewolf",
			"librewolf",
		),
		// auto: try all known paths for both browsers
		_ => (
			"~/.librewolf ~/.config/librewolf/librewolf ~/.mozilla/firefox \
~/.var/app/io.gitlab.librewolf-community/.librewolf ~/.var/app/org.mozilla.firefox/.mozilla/firefox",
			"librewolf 2>/dev/null || firefox",
		),
	}
}

/// Extract a Reddit bearer token and build a matching Firefox User-Agent by
/// SSHing to the remote machine and querying its browser cookies.sqlite via sqlite3.
///
/// Returns `(bearer_token, user_agent_string)`.
async fn ssh_extract_token(host: &str, user: &str, key_path: &str, browser: &str) -> Result<(String, String), String> {
	let (find_dirs, version_bin) = browser_script_parts(browser);
	let remote_script = format!(
		r#"set -e
DB=$(find {find_dirs} -name 'cookies.sqlite' 2>/dev/null | head -1)
[ -z "$DB" ] && echo "ERROR=no cookies.sqlite found in {find_dirs}" && exit 1
CP=$(mktemp) && cp "$DB" "$CP" && trap "rm -f $CP" EXIT
TOKEN=$(sqlite3 "$CP" 'SELECT value FROM moz_cookies WHERE host='\''.reddit.com'\'' AND name='\''token_v2'\'' ORDER BY lastAccessed DESC LIMIT 1' 2>/dev/null)
[ -z "$TOKEN" ] && echo "ERROR=no token_v2 cookie found for .reddit.com" && exit 1
echo "TOKEN=$TOKEN"
VERSION=$({version_bin} --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+' | head -1)
ARCH=$(uname -m)
echo "VERSION=$VERSION"
echo "ARCH=$ARCH""#,
		find_dirs = find_dirs,
		version_bin = version_bin,
	);

	let timeout_secs = ssh_timeout();
	let strict_checking = ssh_strict_host_key_checking();
	let output = tokio::time::timeout(
		std::time::Duration::from_secs(timeout_secs + 10),
		tokio::process::Command::new("ssh")
			.args([
				"-i",
				key_path,
				"-o",
				"BatchMode=yes",
				"-o",
				&format!("ConnectTimeout={timeout_secs}"),
				"-o",
				if strict_checking {
					"StrictHostKeyChecking=yes"
				} else {
					"StrictHostKeyChecking=accept-new"
				},
				&format!("{user}@{host}"),
				&remote_script,
			])
			.output(),
	)
	.await
	.map_err(|e| format!("SSH command timed out after {timeout_secs} seconds: {e}"))?
	.map_err(|e| format!("Failed to run ssh: {e}"))?;

	let stdout = String::from_utf8_lossy(&output.stdout);
	let stderr = String::from_utf8_lossy(&output.stderr);

	if !output.status.success() {
		// Check if stdout has an ERROR= line (from the script)
		for line in stdout.lines() {
			if let Some(msg) = line.strip_prefix("ERROR=") {
				return Err(msg.to_string());
			}
		}
		let code = output.status.code().unwrap_or(-1);
		log::error!(
			"SSH extraction (key-only) failed exit={code} stderr_len={} stderr=\"{}\" stdout_preview=\"{}\"",
			stderr.len(),
			stderr.replace('\n', " "),
			stdout.lines().take(3).collect::<Vec<_>>().join(" ").chars().take(200).collect::<String>()
		);
		let hint = if stderr.is_empty() { String::new() } else { format!(" — {stderr}") };
		return Err(format!("SSH command failed (exit {}){hint}", code));
	}

	// Parse key=value lines
	let mut token_raw = String::new();
	let mut version = String::new();
	let mut arch = String::new();

	for line in stdout.lines() {
		if let Some(v) = line.strip_prefix("TOKEN=") {
			token_raw = v.to_string();
		} else if let Some(v) = line.strip_prefix("VERSION=") {
			version = v.to_string();
		} else if let Some(v) = line.strip_prefix("ARCH=") {
			arch = v.to_string();
		} else if let Some(msg) = line.strip_prefix("ERROR=") {
			return Err(msg.to_string());
		}
	}

	if token_raw.is_empty() {
		return Err("No token found in SSH output".to_string());
	}

	// Decode the JWT payload to extract the bearer token
	let bearer = decode_browser_token(&token_raw).unwrap_or_else(|| {
		log::warn!("token_v2 could not be decoded as JWT; using raw value");
		token_raw
	});

	// Build a Firefox-compatible UA string from the detected version and arch
	let ua = if !version.is_empty() && !arch.is_empty() {
		let ua_arch = match arch.as_str() {
			"amd64" | "x86_64" => "x86_64",
			"aarch64" | "arm64" => "aarch64",
			"i686" | "i386" => "i686",
			other => other,
		};
		let major = version.split('.').next().unwrap_or(&version);
		format!("Mozilla/5.0 (X11; Linux {ua_arch}; rv:{version}) Gecko/20100101 Firefox/{major}.0")
	} else {
		String::new()
	};

	Ok((bearer, ua))
}

/// Same as ssh_extract_token but uses sshpass to supply the key passphrase (for encrypted private keys).
/// Runs: sshpass -p passphrase ssh -i key_path -o BatchMode=no ...
async fn ssh_extract_token_key_passphrase(host: &str, user: &str, key_path: &str, passphrase: &str, browser: &str) -> Result<(String, String), String> {
	let (find_dirs, version_bin) = browser_script_parts(browser);
	let remote_script = format!(
		r#"set -e
DB=$(find {find_dirs} -name 'cookies.sqlite' 2>/dev/null | head -1)
[ -z "$DB" ] && echo "ERROR=no cookies.sqlite found in {find_dirs}" && exit 1
CP=$(mktemp) && cp "$DB" "$CP" && trap "rm -f $CP" EXIT
TOKEN=$(sqlite3 "$CP" 'SELECT value FROM moz_cookies WHERE host='\''.reddit.com'\'' AND name='\''token_v2'\'' ORDER BY lastAccessed DESC LIMIT 1' 2>/dev/null)
[ -z "$TOKEN" ] && echo "ERROR=no token_v2 cookie found for .reddit.com" && exit 1
echo "TOKEN=$TOKEN"
VERSION=$({version_bin} --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+' | head -1)
ARCH=$(uname -m)
echo "VERSION=$VERSION"
echo "ARCH=$ARCH""#,
		find_dirs = find_dirs,
		version_bin = version_bin,
	);

	// BatchMode=no so ssh will prompt for key passphrase and sshpass can supply it
	let timeout_secs = ssh_timeout();
	let strict_checking = ssh_strict_host_key_checking();
	let output = tokio::time::timeout(
		std::time::Duration::from_secs(timeout_secs + 10),
		tokio::process::Command::new("sshpass")
			.args([
				"-p",
				passphrase,
				"ssh",
				"-i",
				key_path,
				"-o",
				"BatchMode=no",
				"-o",
				&format!("ConnectTimeout={timeout_secs}"),
				"-o",
				if strict_checking {
					"StrictHostKeyChecking=yes"
				} else {
					"StrictHostKeyChecking=accept-new"
				},
				&format!("{user}@{host}"),
				&remote_script,
			])
			.output(),
	)
	.await
	.map_err(|e| format!("SSH command timed out after {timeout_secs} seconds: {e}"))?
	.map_err(|e| format!("Failed to run sshpass/ssh: {e}. Is sshpass installed?"))?;

	let stdout = String::from_utf8_lossy(&output.stdout);
	let stderr = String::from_utf8_lossy(&output.stderr);

	if !output.status.success() {
		for line in stdout.lines() {
			if let Some(msg) = line.strip_prefix("ERROR=") {
				return Err(msg.to_string());
			}
		}
		let code = output.status.code().unwrap_or(-1);
		log::error!(
			"SSH extraction (key+passphrase) failed exit={code} stderr_len={} stderr=\"{}\" stdout_preview=\"{}\"",
			stderr.len(),
			stderr.replace('\n', " "),
			stdout.lines().take(3).collect::<Vec<_>>().join(" ").chars().take(200).collect::<String>()
		);
		let hint = if stderr.is_empty() { String::new() } else { format!(" — {stderr}") };
		return Err(format!("SSH command failed (exit {}){hint}", code));
	}

	let mut token_raw = String::new();
	let mut version = String::new();
	let mut arch = String::new();
	for line in stdout.lines() {
		if let Some(v) = line.strip_prefix("TOKEN=") {
			token_raw = v.to_string();
		} else if let Some(v) = line.strip_prefix("VERSION=") {
			version = v.to_string();
		} else if let Some(v) = line.strip_prefix("ARCH=") {
			arch = v.to_string();
		} else if let Some(msg) = line.strip_prefix("ERROR=") {
			return Err(msg.to_string());
		}
	}
	if token_raw.is_empty() {
		return Err("No token found in SSH output".to_string());
	}

	let bearer = decode_browser_token(&token_raw).unwrap_or_else(|| {
		log::warn!("token_v2 could not be decoded as JWT; using raw value");
		token_raw
	});
	let ua = if !version.is_empty() && !arch.is_empty() {
		let ua_arch = match arch.as_str() {
			"amd64" | "x86_64" => "x86_64",
			"aarch64" | "arm64" => "aarch64",
			"i686" | "i386" => "i686",
			other => other,
		};
		let major = version.split('.').next().unwrap_or(&version);
		format!("Mozilla/5.0 (X11; Linux {ua_arch}; rv:{version}) Gecko/20100101 Firefox/{major}.0")
	} else {
		String::new()
	};
	Ok((bearer, ua))
}

/// Same as ssh_extract_token but authenticates with password via sshpass.
async fn ssh_extract_token_with_password(host: &str, user: &str, password: &str, browser: &str) -> Result<(String, String), String> {
	let (find_dirs, version_bin) = browser_script_parts(browser);
	let remote_script = format!(
		r#"set -e
DB=$(find {find_dirs} -name 'cookies.sqlite' 2>/dev/null | head -1)
[ -z "$DB" ] && echo "ERROR=no cookies.sqlite found in {find_dirs}" && exit 1
CP=$(mktemp) && cp "$DB" "$CP" && trap "rm -f $CP" EXIT
TOKEN=$(sqlite3 "$CP" 'SELECT value FROM moz_cookies WHERE host='\''.reddit.com'\'' AND name='\''token_v2'\'' ORDER BY lastAccessed DESC LIMIT 1' 2>/dev/null)
[ -z "$TOKEN" ] && echo "ERROR=no token_v2 cookie found for .reddit.com" && exit 1
echo "TOKEN=$TOKEN"
VERSION=$({version_bin} --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+' | head -1)
ARCH=$(uname -m)
echo "VERSION=$VERSION"
echo "ARCH=$ARCH""#,
		find_dirs = find_dirs,
		version_bin = version_bin,
	);

	let timeout_secs = ssh_timeout();
	let strict_checking = ssh_strict_host_key_checking();
	let output = tokio::time::timeout(
		std::time::Duration::from_secs(timeout_secs + 10),
		tokio::process::Command::new("sshpass")
			.args([
				"-p",
				password,
				"ssh",
				"-o",
				"BatchMode=yes",
				"-o",
				&format!("ConnectTimeout={timeout_secs}"),
				"-o",
				if strict_checking {
					"StrictHostKeyChecking=yes"
				} else {
					"StrictHostKeyChecking=accept-new"
				},
				&format!("{user}@{host}"),
				&remote_script,
			])
			.output(),
	)
	.await
	.map_err(|e| format!("SSH command timed out after {timeout_secs} seconds: {e}"))?
	.map_err(|e| format!("Failed to run sshpass/ssh: {e}. Is sshpass installed?"))?;

	let stdout = String::from_utf8_lossy(&output.stdout);
	let stderr = String::from_utf8_lossy(&output.stderr);

	if !output.status.success() {
		for line in stdout.lines() {
			if let Some(msg) = line.strip_prefix("ERROR=") {
				return Err(msg.to_string());
			}
		}
		let code = output.status.code().unwrap_or(-1);
		log::error!(
			"SSH extraction (password) failed exit={code} stderr_len={} stderr=\"{}\" stdout_preview=\"{}\"",
			stderr.len(),
			stderr.replace('\n', " "),
			stdout.lines().take(3).collect::<Vec<_>>().join(" ").chars().take(200).collect::<String>()
		);
		let hint = if stderr.is_empty() { String::new() } else { format!(" — {stderr}") };
		return Err(format!("SSH command failed (exit {}){hint}", code));
	}

	let mut token_raw = String::new();
	let mut version = String::new();
	let mut arch = String::new();
	for line in stdout.lines() {
		if let Some(v) = line.strip_prefix("TOKEN=") {
			token_raw = v.to_string();
		} else if let Some(v) = line.strip_prefix("VERSION=") {
			version = v.to_string();
		} else if let Some(v) = line.strip_prefix("ARCH=") {
			arch = v.to_string();
		} else if let Some(msg) = line.strip_prefix("ERROR=") {
			return Err(msg.to_string());
		}
	}
	if token_raw.is_empty() {
		return Err("No token found in SSH output".to_string());
	}

	let bearer = decode_browser_token(&token_raw).unwrap_or_else(|| {
		log::warn!("token_v2 could not be decoded as JWT; using raw value");
		token_raw
	});
	let ua = if !version.is_empty() && !arch.is_empty() {
		let ua_arch = match arch.as_str() {
			"amd64" | "x86_64" => "x86_64",
			"aarch64" | "arm64" => "aarch64",
			"i686" | "i386" => "i686",
			other => other,
		};
		let major = version.split('.').next().unwrap_or(&version);
		format!("Mozilla/5.0 (X11; Linux {ua_arch}; rv:{version}) Gecko/20100101 Firefox/{major}.0")
	} else {
		String::new()
	};
	Ok((bearer, ua))
}

/// `GET /auth/callback` — handle Reddit's OAuth redirect, exchange the code
/// for tokens, and set the encrypted session cookie.
pub async fn oauth_callback(req: Request<Body>) -> Result<Response<Body>, String> {
	let client_id = CONFIG.oauth_client_id.clone().ok_or("REDLIB_OAUTH_CLIENT_ID is not configured")?;
	let client_secret = CONFIG.oauth_client_secret.clone().ok_or("REDLIB_OAUTH_CLIENT_SECRET is not configured")?;
	let redirect_uri = CONFIG.oauth_redirect_uri.clone().ok_or("REDLIB_OAUTH_REDIRECT_URI is not configured")?;

	// Parse query parameters from the callback URL
	let query = req.uri().query().unwrap_or("");
	let params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes()).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();

	// Validate CSRF state
	let state = params.get("state").ok_or("Missing 'state' parameter in callback")?;
	let csrf_cookie = req.cookie(csrf_cookie_name()).ok_or("Missing CSRF cookie — possible CSRF attack or cookie expired")?;
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
	// Populate Reddit subscriptions for Feeds nav (fetch before moving tokens into session)
	let reddit_subs: Vec<String> = client::fetch_subscribed_subreddits_with_bearer(&tokens.access_token).await.unwrap_or_default();

	let expires_at = OffsetDateTime::now_utc().unix_timestamp() + tokens.expires_in as i64;

	let session = SessionData {
		access_token: tokens.access_token,
		refresh_token: tokens.refresh_token,
		username,
		expires_at,
		csrf_token: Uuid::new_v4().to_string(),
	};

	let mut response = add_session_to_vault(session)?;

	if !reddit_subs.is_empty() {
		response.insert_cookie(
			Cookie::build((subscriptions_cookie_name(), reddit_subs.join("+")))
				.path("/")
				.http_only(true)
				.secure(secure_cookies())
				.same_site(SameSite::Lax)
				.expires(OffsetDateTime::now_utc() + Duration::weeks(4))
				.into(),
		);
	}
	// Clear the CSRF cookie — it's served its purpose
	response.remove_cookie(csrf_cookie_name().to_string());
	Ok(response)
}

/// `POST /logout` — validate CSRF token, remove active session from vault.
pub async fn logout(req: Request<Body>) -> Result<Response<Body>, String> {
	// Extract auth context before consuming the body
	let auth = AuthContext::from_request(&req);

	// Get username early before consuming the request
	let username = auth.username().map(|s| s.to_string());

	// Get session cookie before consuming request (clone to own the value)
	let vault_cookie = req.cookie(session_cookie_name()).map(|c| c.value().to_string());

	// Read and parse POST body for CSRF token (with size limit)
	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();

	let submitted_csrf = form.get("csrf_token").map(|s| s.as_str()).unwrap_or("");
	validate_csrf_token(&auth, submitted_csrf)?;

	// Remove active session from vault
	if let Some(username) = username {
		return remove_session_from_vault(&username, vault_cookie.as_deref());
	}

	let mut response = redirect(AUTH_LANDING_PATH);
	response.remove_cookie(session_cookie_name().to_string());
	response.remove_cookie(active_session_cookie_name().to_string());
	response.remove_cookie(subscriptions_cookie_name().to_string());
	Ok(response)
}

/// `POST /auth/switch` — switch to a different account in the vault.
/// Body: username=...
pub async fn switch_account(req: Request<Body>) -> Result<Response<Body>, String> {
	// Get session cookie before consuming request (clone to own the value)
	let vault_cookie = req.cookie(session_cookie_name()).map(|c| c.value().to_string());

	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();

	let username = form.get("username").map(|s| s.as_str()).unwrap_or("");
	if username.is_empty() {
		return Err("Username is required".to_string());
	}

	switch_active_session(username, vault_cookie.as_deref())
}

/// `POST /auth/remove` — remove a specific account from the vault.
/// Body: username=...
pub async fn remove_account(req: Request<Body>) -> Result<Response<Body>, String> {
	// Get session cookie before consuming request (clone to own the value)
	let vault_cookie = req.cookie(session_cookie_name()).map(|c| c.value().to_string());

	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	if body_bytes.len() > MAX_BODY_SIZE {
		return Err("Request body too large".to_string());
	}
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();

	let username = form.get("username").map(|s| s.as_str()).unwrap_or("");
	if username.is_empty() {
		return Err("Username is required".to_string());
	}

	remove_session_from_vault(username, vault_cookie.as_deref())
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

	let new_token = json["access_token"].as_str().ok_or("Missing access_token in refresh response")?.to_string();
	let expires_in = json["expires_in"].as_i64().unwrap_or(3600);
	let expires_at = OffsetDateTime::now_utc().unix_timestamp() + expires_in;

	Ok((new_token, expires_at))
}

/// `POST /action/sync_subscriptions` — re-fetch the user's Reddit subscriptions
/// from `/subreddits/mine/subscriber` and refresh the subscriptions cookie.
pub async fn sync_subscriptions(req: Request<Body>) -> Result<Response<Body>, String> {
	let auth = AuthContext::from_request(&req);
	if !auth.is_authenticated() {
		return Err("Not logged in".to_string());
	}

	let subs = client::fetch_subscribed_subreddits(&auth).await.unwrap_or_default();

	let back = req.headers().get("Referer").and_then(|v| v.to_str().ok()).unwrap_or("/").to_string();

	let mut response = crate::utils::redirect(&back);

	if !subs.is_empty() {
		response.insert_cookie(
			Cookie::build((subscriptions_cookie_name(), subs.join("+")))
				.path("/")
				.http_only(true)
				.secure(secure_cookies())
				.same_site(SameSite::Lax)
				.expires(OffsetDateTime::now_utc() + Duration::weeks(4))
				.into(),
		);
	}

	Ok(response)
}
