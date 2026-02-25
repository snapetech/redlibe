use std::fs;
use std::path::{Path, PathBuf};

use redlib::token_import;
use rusqlite::Connection;
use uuid::Uuid;

#[test]
fn firefox_discover_profiles_and_import_local_token() {
	let root = test_dir("firefox");
	let profile = root.join("abcd.default-release");
	fs::create_dir_all(&profile).unwrap();
	create_firefox_cookie_db(&profile.join("cookies.sqlite"), "header.payload.sig");

	let profiles = token_import::firefox::discover_profiles_in_base("firefox", &root);
	assert_eq!(profiles.len(), 1);
	assert_eq!(profiles[0].browser, "firefox");
	assert!(profiles[0].id.contains("firefox:abcd.default-release"));

	// Use manual path for deterministic import in CI/tests.
	let imported = token_import::import_local("firefox", None, Some(profile.to_str().unwrap())).unwrap();
	assert_eq!(imported.bearer_token, "header.payload.sig");
}

#[test]
fn firefox_import_errors_when_no_cookie_present() {
	let root = test_dir("firefox-empty");
	let profile = root.join("empty.default");
	fs::create_dir_all(&profile).unwrap();
	create_firefox_cookie_db_without_token(&profile.join("cookies.sqlite"));

	let err = token_import::import_local("firefox", None, Some(profile.to_str().unwrap())).unwrap_err();
	assert!(err.contains("token_v2"), "unexpected error: {err}");
}

#[test]
fn chromium_discover_profiles_and_import_plaintext_cookie() {
	let root = test_dir("chromium");
	let profile = root.join("Default");
	fs::create_dir_all(&profile).unwrap();
	create_chromium_cookie_db(&profile.join("Cookies"), "jwt.part.sig", &[]);

	let profiles = token_import::chromium::discover_profiles_in_base("chrome", &root);
	assert_eq!(profiles.len(), 1);
	assert_eq!(profiles[0].browser, "chrome");
	assert_eq!(profiles[0].id, "chrome:Default");

	let imported = token_import::import_local("chrome", None, Some(profile.to_str().unwrap())).unwrap();
	assert_eq!(imported.bearer_token, "jwt.part.sig");
}

#[test]
fn chromium_import_returns_clear_error_for_undecryptable_cookie() {
	let root = test_dir("chromium-encrypted");
	let profile = root.join("Default");
	fs::create_dir_all(&profile).unwrap();
	create_chromium_cookie_db(&profile.join("Cookies"), "", b"v10notreallyencrypted");

	let err = token_import::import_local("chrome", None, Some(profile.to_str().unwrap())).unwrap_err();
	assert!(
		err.contains("encrypted") || err.contains("decrypted"),
		"unexpected error: {err}"
	);
}

#[test]
fn chromium_profile_selection_rejects_missing_profile_id() {
	let err = token_import::import_local("chrome", Some("chrome:missing"), None).unwrap_err();
	assert!(err.contains("Selected profile was not found") || err.contains("No Chrome profiles"), "unexpected error: {err}");
}

fn create_firefox_cookie_db(path: &Path, token: &str) {
	let conn = Connection::open(path).unwrap();
	conn.execute_batch(
		"CREATE TABLE moz_cookies (
			id INTEGER PRIMARY KEY,
			originAttributes TEXT,
			name TEXT,
			value TEXT,
			host TEXT,
			path TEXT,
			expiry INTEGER,
			lastAccessed INTEGER,
			creationTime INTEGER,
			isSecure INTEGER,
			isHttpOnly INTEGER,
			inBrowserElement INTEGER DEFAULT 0,
			sameSite INTEGER DEFAULT 0,
			rawSameSite INTEGER DEFAULT 0,
			schemeMap INTEGER DEFAULT 0
		);",
	)
	.unwrap();
	conn.execute(
		"INSERT INTO moz_cookies (name, value, host, path, expiry, lastAccessed, creationTime, isSecure, isHttpOnly)
		 VALUES (?1, ?2, '.reddit.com', '/', 9999999999, 2, 1, 1, 1)",
		("token_v2", token),
	)
	.unwrap();
}

fn create_firefox_cookie_db_without_token(path: &Path) {
	let conn = Connection::open(path).unwrap();
	conn.execute_batch(
		"CREATE TABLE moz_cookies (
			id INTEGER PRIMARY KEY,
			originAttributes TEXT,
			name TEXT,
			value TEXT,
			host TEXT,
			path TEXT,
			expiry INTEGER,
			lastAccessed INTEGER,
			creationTime INTEGER,
			isSecure INTEGER,
			isHttpOnly INTEGER
		);",
	)
	.unwrap();
	conn.execute(
		"INSERT INTO moz_cookies (name, value, host, path, expiry, lastAccessed, creationTime, isSecure, isHttpOnly)
		 VALUES ('other', 'x', '.reddit.com', '/', 9999999999, 2, 1, 1, 1)",
		[],
	)
	.unwrap();
}

fn create_chromium_cookie_db(path: &Path, value: &str, encrypted_value: &[u8]) {
	let conn = Connection::open(path).unwrap();
	conn.execute_batch(
		"CREATE TABLE cookies (
			creation_utc INTEGER NOT NULL DEFAULT 0,
			host_key TEXT NOT NULL,
			top_frame_site_key TEXT NOT NULL DEFAULT '',
			name TEXT NOT NULL,
			value TEXT NOT NULL DEFAULT '',
			encrypted_value BLOB NOT NULL DEFAULT x'',
			path TEXT NOT NULL DEFAULT '/',
			expires_utc INTEGER NOT NULL DEFAULT 0,
			is_secure INTEGER NOT NULL DEFAULT 0,
			is_httponly INTEGER NOT NULL DEFAULT 0,
			last_access_utc INTEGER NOT NULL DEFAULT 0,
			has_expires INTEGER NOT NULL DEFAULT 0,
			is_persistent INTEGER NOT NULL DEFAULT 0,
			priority INTEGER NOT NULL DEFAULT 1,
			samesite INTEGER NOT NULL DEFAULT 0,
			source_scheme INTEGER NOT NULL DEFAULT 0,
			source_port INTEGER NOT NULL DEFAULT 0,
			last_update_utc INTEGER NOT NULL DEFAULT 0,
			source_type INTEGER NOT NULL DEFAULT 0
		);",
	)
	.unwrap();
	conn.execute(
		"INSERT INTO cookies (host_key, name, value, encrypted_value, last_access_utc)
		 VALUES (?1, ?2, ?3, ?4, ?5)",
		(".reddit.com", "token_v2", value, encrypted_value, 10_i64),
	)
	.unwrap();
}

fn test_dir(prefix: &str) -> PathBuf {
	let dir = std::env::temp_dir().join(format!("redlib-test-{prefix}-{}", Uuid::new_v4()));
	fs::create_dir_all(&dir).unwrap();
	dir
}
