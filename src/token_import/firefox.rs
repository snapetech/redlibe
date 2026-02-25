use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use uuid::Uuid;

use super::{ImportedBrowserSession, LocalProfile};

pub fn discover_profiles(browser: &str) -> Vec<LocalProfile> {
	let Some(base) = firefox_base_dir(browser) else {
		return Vec::new();
	};
	discover_profiles_in_base(browser, &base)
}

pub fn discover_profiles_in_base(browser: &str, base: &Path) -> Vec<LocalProfile> {
	let mut out = Vec::new();

	let entries = match fs::read_dir(base) {
		Ok(v) => v,
		Err(_) => return out,
	};

	for entry in entries.flatten() {
		let path = entry.path();
		if !path.is_dir() {
			continue;
		}
		let cookies = path.join("cookies.sqlite");
		if !cookies.is_file() {
			continue;
		}
		let profile_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("profile");
		let id = format!("{browser}:{profile_name}");
		let label = format!("{} ({profile_name})", display_browser(browser));
		out.push(LocalProfile { browser: browser.to_string(), id, label, path });
	}

	out
}

pub fn import_token(browser: &str, profile_id: Option<&str>, profile_path: Option<&str>) -> Result<ImportedBrowserSession, String> {
	let profile = resolve_profile(browser, profile_id, profile_path)?;
	let db_path = profile.join("cookies.sqlite");
	if !db_path.is_file() {
		return Err(format!("cookies.sqlite not found in {}", profile.display()));
	}
	let tmp = copy_to_temp(&db_path)?;
	let _cleanup = TempFileCleanup(tmp.clone());
	let conn = Connection::open(&tmp).map_err(|e| format!("Failed to open cookie DB: {e}"))?;
	let token = read_firefox_token(&conn)?;
	let ua = read_firefox_user_agent(browser);
	Ok(ImportedBrowserSession {
		bearer_token: token,
		user_agent: ua,
	})
}

struct TempFileCleanup(PathBuf);
impl Drop for TempFileCleanup {
	fn drop(&mut self) {
		let _ = fs::remove_file(&self.0);
	}
}

fn copy_to_temp(path: &Path) -> Result<PathBuf, String> {
	let tmp = env::temp_dir().join(format!("redlib-firefox-cookies-{}.sqlite", Uuid::new_v4()));
	fs::copy(path, &tmp).map_err(|e| format!("Failed to copy cookie DB: {e}"))?;
	Ok(tmp)
}

fn read_firefox_token(conn: &Connection) -> Result<String, String> {
	let mut stmt = conn
		.prepare(
			"SELECT value
			 FROM moz_cookies
			 WHERE name = 'token_v2'
			   AND (host = '.reddit.com' OR host = 'reddit.com' OR host = 'www.reddit.com')
			 ORDER BY lastAccessed DESC
			 LIMIT 1",
		)
		.map_err(|e| format!("Failed to prepare cookie query: {e}"))?;

	stmt.query_row([], |row| row.get::<_, String>(0))
		.map_err(|_| "No reddit token_v2 cookie found in selected Firefox profile".to_string())
}

fn resolve_profile(browser: &str, profile_id: Option<&str>, profile_path: Option<&str>) -> Result<PathBuf, String> {
	if let Some(path) = profile_path.map(str::trim).filter(|s| !s.is_empty()) {
		return Ok(PathBuf::from(path));
	}

	if let Some(id) = profile_id {
		for p in discover_profiles(browser) {
			if p.id == id {
				return Ok(p.path);
			}
		}
		return Err("Selected profile was not found".to_string());
	}

	discover_profiles(browser)
		.into_iter()
		.next()
		.map(|p| p.path)
		.ok_or_else(|| format!("No {} profiles with cookies.sqlite were found", display_browser(browser)))
}

fn read_firefox_user_agent(browser: &str) -> Option<String> {
	let version = detect_browser_version(browser)?;
	let major = version.split('.').next().unwrap_or(&version);
	Some(format!(
		"Mozilla/5.0 (X11; Linux x86_64; rv:{version}) Gecko/20100101 Firefox/{major}.0"
	))
}

fn detect_browser_version(browser: &str) -> Option<String> {
	let candidates: &[&str] = match browser {
		"librewolf" => &["librewolf"],
		_ => &["firefox"],
	};
	for cmd in candidates {
		let output = std::process::Command::new(cmd).arg("--version").output().ok()?;
		let stdout = String::from_utf8_lossy(&output.stdout);
		for word in stdout.split_whitespace() {
			if word.chars().next().is_some_and(|c| c.is_ascii_digit()) {
				return Some(word.to_string());
			}
		}
	}
	None
}

fn display_browser(browser: &str) -> &'static str {
	match browser {
		"librewolf" => "LibreWolf",
		_ => "Firefox",
	}
}

fn firefox_base_dir(browser: &str) -> Option<PathBuf> {
	if cfg!(target_os = "windows") {
		let appdata = env::var_os("APPDATA")?;
		let mut p = PathBuf::from(appdata);
		match browser {
			"librewolf" => {
				p.push("LibreWolf");
				p.push("Profiles");
			}
			_ => {
				p.push("Mozilla");
				p.push("Firefox");
				p.push("Profiles");
			}
		}
		return Some(p);
	}

	if cfg!(target_os = "macos") {
		let home = env::var_os("HOME")?;
		let mut p = PathBuf::from(home);
		p.push("Library");
		p.push("Application Support");
		match browser {
			"librewolf" => {
				p.push("LibreWolf");
				p.push("Profiles");
			}
			_ => {
				p.push("Firefox");
				p.push("Profiles");
			}
		}
		return Some(p);
	}

	let home = env::var_os("HOME")?;
	let mut p = PathBuf::from(home);
	match browser {
		"librewolf" => p.push(".librewolf"),
		_ => {
			p.push(".mozilla");
			p.push("firefox");
		}
	}
	Some(p)
}
