use std::path::PathBuf;

pub mod chromium;
pub mod firefox;

#[derive(Debug, Clone)]
pub struct ImportedBrowserSession {
	pub bearer_token: String,
	pub user_agent: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LocalProfile {
	pub browser: String,
	pub id: String,
	pub label: String,
	pub path: PathBuf,
}

pub fn discover_local_profiles() -> Vec<LocalProfile> {
	let mut profiles = Vec::new();
	profiles.extend(firefox::discover_profiles("firefox"));
	profiles.extend(firefox::discover_profiles("librewolf"));
	profiles.extend(chromium::discover_profiles("chrome"));
	profiles.extend(chromium::discover_profiles("edge"));
	profiles.sort_by(|a, b| a.label.cmp(&b.label));
	profiles
}

pub fn import_local(browser: &str, profile_id: Option<&str>, profile_path: Option<&str>) -> Result<ImportedBrowserSession, String> {
	match browser {
		"firefox" | "librewolf" => firefox::import_token(browser, profile_id, profile_path),
		"chrome" | "edge" => chromium::import_token(browser, profile_id, profile_path),
		other => Err(format!("Unsupported browser: {other}")),
	}
}
