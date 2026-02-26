use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::{env::var, fs::read_to_string, sync::LazyLock};

/// This is the local static that is initialized at runtime (technically at
/// first request) and contains the instance settings.
pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);

/// This serves as the frontend for an archival API - on removed comments, this URL
/// will be the base of a link, to display removed content (on another site).
pub const DEFAULT_PUSHSHIFT_FRONTEND: &str = "undelete.pullpush.io";

/// Default Firefox user-agent string used for all outbound requests to Reddit.
/// Override with the `REDLIB_USER_AGENT` environment variable.
pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0";

/// Runtime-overridable User-Agent. Set by the SSH import flow to match the
/// remote browser's exact UA. Takes priority over `REDLIB_USER_AGENT` and the
/// compiled-in default. Initialized to an empty string (= not set).
pub static RUNTIME_USER_AGENT: LazyLock<ArcSwap<String>> = LazyLock::new(|| ArcSwap::from_pointee(String::new()));

/// Returns the effective outbound User-Agent string. Priority:
/// 1. `RUNTIME_USER_AGENT` (set at runtime by SSH import)
/// 2. `REDLIB_USER_AGENT` env/config
/// 3. `DEFAULT_USER_AGENT`
pub fn get_user_agent() -> String {
	let runtime = RUNTIME_USER_AGENT.load();
	if !runtime.is_empty() {
		return (**runtime).clone();
	}
	CONFIG.user_agent.clone().unwrap_or_else(|| DEFAULT_USER_AGENT.to_string())
}

/// Override the outbound User-Agent at runtime (e.g. after SSH import).
pub fn set_runtime_user_agent(ua: String) {
	RUNTIME_USER_AGENT.store(Arc::new(ua));
}

/// Stores the configuration parsed from the environment variables and the
/// config file. `Config::Default()` contains None for each setting.
/// When adding more config settings, add it to `Config::load`,
/// `get_setting_from_config`, both below, as well as
/// `instance_info::InstanceInfo.to_string`(), README.md and app.json.
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Config {
	#[serde(rename = "REDLIB_SFW_ONLY")]
	#[serde(alias = "LIBREDDIT_SFW_ONLY")]
	pub(crate) sfw_only: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_THEME")]
	#[serde(alias = "LIBREDDIT_DEFAULT_THEME")]
	pub(crate) default_theme: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_FRONT_PAGE")]
	#[serde(alias = "LIBREDDIT_DEFAULT_FRONT_PAGE")]
	pub(crate) default_front_page: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_LAYOUT")]
	#[serde(alias = "LIBREDDIT_DEFAULT_LAYOUT")]
	pub(crate) default_layout: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_WIDE")]
	#[serde(alias = "LIBREDDIT_DEFAULT_WIDE")]
	pub(crate) default_wide: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_COMMENT_SORT")]
	#[serde(alias = "LIBREDDIT_DEFAULT_COMMENT_SORT")]
	pub(crate) default_comment_sort: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_POST_SORT")]
	#[serde(alias = "LIBREDDIT_DEFAULT_POST_SORT")]
	pub(crate) default_post_sort: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_BLUR_SPOILER")]
	#[serde(alias = "LIBREDDIT_DEFAULT_BLUR_SPOILER")]
	pub(crate) default_blur_spoiler: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_SHOW_NSFW")]
	#[serde(alias = "LIBREDDIT_DEFAULT_SHOW_NSFW")]
	pub(crate) default_show_nsfw: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_BLUR_NSFW")]
	#[serde(alias = "LIBREDDIT_DEFAULT_BLUR_NSFW")]
	pub(crate) default_blur_nsfw: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_USE_HLS")]
	#[serde(alias = "LIBREDDIT_DEFAULT_USE_HLS")]
	pub(crate) default_use_hls: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_HIDE_HLS_NOTIFICATION")]
	#[serde(alias = "LIBREDDIT_DEFAULT_HIDE_HLS_NOTIFICATION")]
	pub(crate) default_hide_hls_notification: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_HIDE_AWARDS")]
	#[serde(alias = "LIBREDDIT_DEFAULT_HIDE_AWARDS")]
	pub(crate) default_hide_awards: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_HIDE_SIDEBAR_AND_SUMMARY")]
	#[serde(alias = "LIBREDDIT_DEFAULT_HIDE_SIDEBAR_AND_SUMMARY")]
	pub(crate) default_hide_sidebar_and_summary: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_HIDE_SCORE")]
	#[serde(alias = "LIBREDDIT_DEFAULT_HIDE_SCORE")]
	pub(crate) default_hide_score: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_SUBSCRIPTIONS")]
	#[serde(alias = "LIBREDDIT_DEFAULT_SUBSCRIPTIONS")]
	pub(crate) default_subscriptions: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_FILTERS")]
	#[serde(alias = "LIBREDDIT_DEFAULT_FILTERS")]
	pub(crate) default_filters: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_DISABLE_VISIT_REDDIT_CONFIRMATION")]
	#[serde(alias = "LIBREDDIT_DEFAULT_DISABLE_VISIT_REDDIT_CONFIRMATION")]
	pub(crate) default_disable_visit_reddit_confirmation: Option<String>,

	#[serde(rename = "REDLIB_BANNER")]
	#[serde(alias = "LIBREDDIT_BANNER")]
	pub(crate) banner: Option<String>,

	#[serde(rename = "REDLIB_ROBOTS_DISABLE_INDEXING")]
	#[serde(alias = "LIBREDDIT_ROBOTS_DISABLE_INDEXING")]
	pub(crate) robots_disable_indexing: Option<String>,

	#[serde(rename = "REDLIB_PUSHSHIFT_FRONTEND")]
	#[serde(alias = "LIBREDDIT_PUSHSHIFT_FRONTEND")]
	pub(crate) pushshift: Option<String>,

	#[serde(rename = "REDLIB_ENABLE_RSS")]
	pub(crate) enable_rss: Option<String>,

	#[serde(rename = "REDLIB_FULL_URL")]
	pub(crate) full_url: Option<String>,

	#[serde(rename = "REDLIB_DEFAULT_REMOVE_DEFAULT_FEEDS")]
	pub(crate) default_remove_default_feeds: Option<String>,

	/// Outbound User-Agent for all requests to Reddit. Defaults to a pinned Firefox UA.
	#[serde(rename = "REDLIB_USER_AGENT")]
	pub(crate) user_agent: Option<String>,

	// --- Extended auth config (redlib-extended) ---
	/// Reddit OAuth app client ID (user-registered app for real login flow).
	#[serde(rename = "REDLIB_OAUTH_CLIENT_ID")]
	pub(crate) oauth_client_id: Option<String>,

	/// Reddit OAuth app client secret.
	#[serde(rename = "REDLIB_OAUTH_CLIENT_SECRET")]
	pub(crate) oauth_client_secret: Option<String>,

	/// OAuth redirect URI registered with the Reddit app.
	#[serde(rename = "REDLIB_OAUTH_REDIRECT_URI")]
	pub(crate) oauth_redirect_uri: Option<String>,

	/// 32+ byte secret used to encrypt session cookies (AES-256-GCM key material).
	#[serde(rename = "REDLIB_SESSION_SECRET")]
	pub(crate) session_secret: Option<String>,

	/// Raw Reddit bearer token. When set, all API calls use this token directly,
	/// bypassing the anonymous spoofed-token flow entirely.
	#[serde(rename = "REDLIB_RAW_TOKEN")]
	pub(crate) raw_token: Option<String>,

	/// Browser-exported Reddit token. Accepts the raw `token_v2` cookie value
	/// from a Firefox/LibreWolf session — the JWT payload is decoded to extract
	/// the bearer token. Falls back to treating the value as a raw bearer token
	/// if JWT decoding fails. Lower priority than `REDLIB_RAW_TOKEN`.
	#[serde(rename = "REDLIB_BROWSER_TOKEN")]
	pub(crate) browser_token: Option<String>,

	/// Set to `on` to add the `Secure` attribute to all session and CSRF cookies.
	/// Enable this when running behind HTTPS. Defaults to off (safe for HTTP-only
	/// local setups).
	#[serde(rename = "REDLIB_SECURE_COOKIES")]
	pub(crate) secure_cookies: Option<String>,

	// --- Smart feed / local state (redlib-extended) ---
	/// Set to `on` to enable per-user SQLite local state (read/saved/mutes).
	#[serde(rename = "REDLIB_ENABLE_LOCAL_STATE")]
	pub(crate) enable_local_state: Option<String>,

	/// Path to the SQLite database file for local state. Default: `redlib.sqlite`.
	#[serde(rename = "REDLIB_DB_PATH")]
	pub(crate) db_path: Option<String>,

	// --- SSH browser-token import (redlib-extended) ---
	/// SSH hostname (or alias) of the machine running the browser whose session
	/// to import. Pre-fills the login page form. Default: `kspld0`.
	#[serde(rename = "REDLIB_SSH_HOST")]
	pub(crate) ssh_host: Option<String>,

	/// SSH username on the remote machine. Default: `keith`.
	#[serde(rename = "REDLIB_SSH_USER")]
	pub(crate) ssh_user: Option<String>,

	/// Path to the SSH identity file used for the import connection.
	/// Default: `~/.ssh/id_ed25519`.
	#[serde(rename = "REDLIB_SSH_KEY")]
	pub(crate) ssh_key: Option<String>,

	/// SSH connection timeout in seconds. Default: `15`.
	#[serde(rename = "REDLIB_SSH_TIMEOUT")]
	pub(crate) ssh_timeout: Option<String>,

	/// Whether to verify SSH host keys. When disabled (default), new hosts are
	/// automatically added to known_hosts. When enabled, strict host key checking
	/// is used — the host must already be in known_hosts or connection will fail.
	/// Default: `false` (accept-new).
	#[serde(rename = "REDLIB_SSH_STRICT_HOST_KEY_CHECKING")]
	pub(crate) ssh_strict_host_key_checking: Option<String>,
}

impl Config {
	/// Load the configuration from the environment variables and the config file.
	/// In the case that there are no environment variables set and there is no
	/// config file, this function returns a Config that contains all None values.
	pub fn load() -> Self {
		let load_config = |name: &str| {
			let new_file = read_to_string(name);
			new_file.ok().and_then(|new_file| toml::from_str::<Self>(&new_file).ok())
		};
		let mut config = load_config("redlib.toml").or_else(|| load_config("libreddit.toml"));
		if config.is_none() {
			for path in per_user_config_candidates() {
				let new_file = read_to_string(&path);
				if let Some(parsed) = new_file.ok().and_then(|s| toml::from_str::<Self>(&s).ok()) {
					config = Some(parsed);
					break;
				}
			}
		}
		let config = config.unwrap_or_default();

		// This function defines the order of preference - first check for
		// environment variables with "REDLIB", then check the legacy LIBREDDIT
		// option, then check the config, then if all are `None`, return a `None`
		let parse = |key: &str| -> Option<String> {
			// Return the first non-`None` value
			// If all are `None`, return `None`
			let legacy_key = key.replace("REDLIB_", "LIBREDDIT_");
			var(key).ok().or_else(|| var(legacy_key).ok()).or_else(|| get_setting_from_config(key, &config))
		};
		Self {
			sfw_only: parse("REDLIB_SFW_ONLY"),
			default_theme: parse("REDLIB_DEFAULT_THEME"),
			default_front_page: parse("REDLIB_DEFAULT_FRONT_PAGE"),
			default_layout: parse("REDLIB_DEFAULT_LAYOUT"),
			default_post_sort: parse("REDLIB_DEFAULT_POST_SORT"),
			default_wide: parse("REDLIB_DEFAULT_WIDE"),
			default_comment_sort: parse("REDLIB_DEFAULT_COMMENT_SORT"),
			default_blur_spoiler: parse("REDLIB_DEFAULT_BLUR_SPOILER"),
			default_show_nsfw: parse("REDLIB_DEFAULT_SHOW_NSFW"),
			default_blur_nsfw: parse("REDLIB_DEFAULT_BLUR_NSFW"),
			default_use_hls: parse("REDLIB_DEFAULT_USE_HLS"),
			default_hide_hls_notification: parse("REDLIB_DEFAULT_HIDE_HLS_NOTIFICATION"),
			default_hide_awards: parse("REDLIB_DEFAULT_HIDE_AWARDS"),
			default_hide_sidebar_and_summary: parse("REDLIB_DEFAULT_HIDE_SIDEBAR_AND_SUMMARY"),
			default_hide_score: parse("REDLIB_DEFAULT_HIDE_SCORE"),
			default_subscriptions: parse("REDLIB_DEFAULT_SUBSCRIPTIONS"),
			default_filters: parse("REDLIB_DEFAULT_FILTERS"),
			default_disable_visit_reddit_confirmation: parse("REDLIB_DEFAULT_DISABLE_VISIT_REDDIT_CONFIRMATION"),
			banner: parse("REDLIB_BANNER"),
			robots_disable_indexing: parse("REDLIB_ROBOTS_DISABLE_INDEXING"),
			pushshift: parse("REDLIB_PUSHSHIFT_FRONTEND"),
			enable_rss: parse("REDLIB_ENABLE_RSS"),
			full_url: parse("REDLIB_FULL_URL"),
			default_remove_default_feeds: parse("REDLIB_DEFAULT_REMOVE_DEFAULT_FEEDS"),
			user_agent: parse("REDLIB_USER_AGENT"),
			oauth_client_id: parse("REDLIB_OAUTH_CLIENT_ID"),
			oauth_client_secret: parse("REDLIB_OAUTH_CLIENT_SECRET"),
			oauth_redirect_uri: parse("REDLIB_OAUTH_REDIRECT_URI"),
			session_secret: parse("REDLIB_SESSION_SECRET"),
			raw_token: parse("REDLIB_RAW_TOKEN"),
			browser_token: parse("REDLIB_BROWSER_TOKEN"),
			secure_cookies: parse("REDLIB_SECURE_COOKIES"),
			enable_local_state: parse("REDLIB_ENABLE_LOCAL_STATE"),
			db_path: parse("REDLIB_DB_PATH"),
			ssh_host: parse("REDLIB_SSH_HOST"),
			ssh_user: parse("REDLIB_SSH_USER"),
			ssh_key: parse("REDLIB_SSH_KEY"),
			ssh_timeout: parse("REDLIB_SSH_TIMEOUT"),
			ssh_strict_host_key_checking: parse("REDLIB_SSH_STRICT_HOST_KEY_CHECKING"),
		}
	}
}

fn per_user_config_candidates() -> Vec<PathBuf> {
	let mut out = Vec::new();

	if cfg!(target_os = "windows") {
		if let Some(appdata) = std::env::var_os("APPDATA") {
			let base = PathBuf::from(appdata).join("redlibe");
			out.push(base.join("redlib.toml"));
			out.push(base.join("libreddit.toml"));
		}
		return out;
	}

	if cfg!(target_os = "macos") {
		if let Some(home) = std::env::var_os("HOME") {
			let base = PathBuf::from(home).join("Library").join("Application Support").join("redlibe");
			out.push(base.join("redlib.toml"));
			out.push(base.join("libreddit.toml"));
		}
		return out;
	}

	if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
		let base = PathBuf::from(xdg).join("redlibe");
		out.push(base.join("redlib.toml"));
		out.push(base.join("libreddit.toml"));
	}
	if let Some(home) = std::env::var_os("HOME") {
		let base = PathBuf::from(home).join(".config").join("redlibe");
		out.push(base.join("redlib.toml"));
		out.push(base.join("libreddit.toml"));
	}
	out
}

fn get_setting_from_config(name: &str, config: &Config) -> Option<String> {
	match name {
		"REDLIB_SFW_ONLY" => config.sfw_only.clone(),
		"REDLIB_DEFAULT_THEME" => config.default_theme.clone(),
		"REDLIB_DEFAULT_FRONT_PAGE" => config.default_front_page.clone(),
		"REDLIB_DEFAULT_LAYOUT" => config.default_layout.clone(),
		"REDLIB_DEFAULT_COMMENT_SORT" => config.default_comment_sort.clone(),
		"REDLIB_DEFAULT_POST_SORT" => config.default_post_sort.clone(),
		"REDLIB_DEFAULT_BLUR_SPOILER" => config.default_blur_spoiler.clone(),
		"REDLIB_DEFAULT_SHOW_NSFW" => config.default_show_nsfw.clone(),
		"REDLIB_DEFAULT_BLUR_NSFW" => config.default_blur_nsfw.clone(),
		"REDLIB_DEFAULT_USE_HLS" => config.default_use_hls.clone(),
		"REDLIB_DEFAULT_HIDE_HLS_NOTIFICATION" => config.default_hide_hls_notification.clone(),
		"REDLIB_DEFAULT_WIDE" => config.default_wide.clone(),
		"REDLIB_DEFAULT_HIDE_AWARDS" => config.default_hide_awards.clone(),
		"REDLIB_DEFAULT_HIDE_SIDEBAR_AND_SUMMARY" => config.default_hide_sidebar_and_summary.clone(),
		"REDLIB_DEFAULT_HIDE_SCORE" => config.default_hide_score.clone(),
		"REDLIB_DEFAULT_SUBSCRIPTIONS" => config.default_subscriptions.clone(),
		"REDLIB_DEFAULT_FILTERS" => config.default_filters.clone(),
		"REDLIB_DEFAULT_DISABLE_VISIT_REDDIT_CONFIRMATION" => config.default_disable_visit_reddit_confirmation.clone(),
		"REDLIB_BANNER" => config.banner.clone(),
		"REDLIB_ROBOTS_DISABLE_INDEXING" => config.robots_disable_indexing.clone(),
		"REDLIB_PUSHSHIFT_FRONTEND" => config.pushshift.clone(),
		"REDLIB_ENABLE_RSS" => config.enable_rss.clone(),
		"REDLIB_FULL_URL" => config.full_url.clone(),
		"REDLIB_DEFAULT_REMOVE_DEFAULT_FEEDS" => config.default_remove_default_feeds.clone(),
		"REDLIB_USER_AGENT" => config.user_agent.clone(),
		"REDLIB_OAUTH_CLIENT_ID" => config.oauth_client_id.clone(),
		"REDLIB_OAUTH_CLIENT_SECRET" => config.oauth_client_secret.clone(),
		"REDLIB_OAUTH_REDIRECT_URI" => config.oauth_redirect_uri.clone(),
		"REDLIB_SESSION_SECRET" => config.session_secret.clone(),
		"REDLIB_RAW_TOKEN" => config.raw_token.clone(),
		"REDLIB_BROWSER_TOKEN" => config.browser_token.clone(),
		"REDLIB_SECURE_COOKIES" => config.secure_cookies.clone(),
		"REDLIB_ENABLE_LOCAL_STATE" => config.enable_local_state.clone(),
		"REDLIB_DB_PATH" => config.db_path.clone(),
		"REDLIB_SSH_HOST" => config.ssh_host.clone(),
		"REDLIB_SSH_USER" => config.ssh_user.clone(),
		"REDLIB_SSH_KEY" => config.ssh_key.clone(),
		"REDLIB_SSH_TIMEOUT" => config.ssh_timeout.clone(),
		"REDLIB_SSH_STRICT_HOST_KEY_CHECKING" => config.ssh_strict_host_key_checking.clone(),
		_ => None,
	}
}

/// Retrieves setting from environment variable or config file.
pub fn get_setting(name: &str) -> Option<String> {
	get_setting_from_config(name, &CONFIG)
}

#[cfg(test)]
use {sealed_test::prelude::*, std::fs::write};

#[test]
fn test_deserialize() {
	// Must handle empty input
	let result = toml::from_str::<Config>("");
	assert!(result.is_ok(), "Error: {}", result.unwrap_err());
}

#[test]
#[sealed_test(env = [("REDLIB_SFW_ONLY", "on")])]
fn test_env_var() {
	assert!(crate::utils::sfw_only())
}

#[test]
#[sealed_test]
fn test_config() {
	let config_to_write = r#"REDLIB_DEFAULT_COMMENT_SORT = "best""#;
	write("redlib.toml", config_to_write).unwrap();
	assert_eq!(get_setting("REDLIB_DEFAULT_COMMENT_SORT"), Some("best".into()));
}

#[test]
#[sealed_test]
fn test_config_legacy() {
	let config_to_write = r#"LIBREDDIT_DEFAULT_COMMENT_SORT = "best""#;
	write("libreddit.toml", config_to_write).unwrap();
	assert_eq!(get_setting("REDLIB_DEFAULT_COMMENT_SORT"), Some("best".into()));
}

#[test]
#[sealed_test(env = [("LIBREDDIT_SFW_ONLY", "on")])]
fn test_env_var_legacy() {
	assert!(crate::utils::sfw_only())
}

#[test]
#[sealed_test(env = [("REDLIB_DEFAULT_COMMENT_SORT", "top")])]
fn test_env_config_precedence() {
	let config_to_write = r#"REDLIB_DEFAULT_COMMENT_SORT = "best""#;
	write("redlib.toml", config_to_write).unwrap();
	assert_eq!(get_setting("REDLIB_DEFAULT_COMMENT_SORT"), Some("top".into()))
}

#[test]
#[sealed_test(env = [("REDLIB_DEFAULT_COMMENT_SORT", "top")])]
fn test_alt_env_config_precedence() {
	let config_to_write = r#"REDLIB_DEFAULT_COMMENT_SORT = "best""#;
	write("redlib.toml", config_to_write).unwrap();
	assert_eq!(get_setting("REDLIB_DEFAULT_COMMENT_SORT"), Some("top".into()))
}
#[test]
#[sealed_test(env = [("REDLIB_DEFAULT_SUBSCRIPTIONS", "news+bestof")])]
fn test_default_subscriptions() {
	assert_eq!(get_setting("REDLIB_DEFAULT_SUBSCRIPTIONS"), Some("news+bestof".into()));
}

#[test]
#[sealed_test(env = [("REDLIB_DEFAULT_FILTERS", "news+bestof")])]
fn test_default_filters() {
	assert_eq!(get_setting("REDLIB_DEFAULT_FILTERS"), Some("news+bestof".into()));
}

#[test]
#[sealed_test]
fn test_pushshift() {
	let config_to_write = r#"REDLIB_PUSHSHIFT_FRONTEND = "https://api.pushshift.io""#;
	write("redlib.toml", config_to_write).unwrap();
	assert!(get_setting("REDLIB_PUSHSHIFT_FRONTEND").is_some());
	assert_eq!(get_setting("REDLIB_PUSHSHIFT_FRONTEND"), Some("https://api.pushshift.io".into()));
}
