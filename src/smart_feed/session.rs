use crate::config::get_setting;
use crate::server::{RequestExt, ResponseExt};
use cookie::Cookie;
use hyper::{Body, Request, Response};

pub fn local_state_enabled() -> bool {
	matches!(get_setting("REDLIB_ENABLE_LOCAL_STATE"), Some(v) if v == "on")
}

pub fn ensure_sid(req: &Request<Body>, res: &mut Response<Body>) -> Option<String> {
	if !local_state_enabled() {
		return None;
	}
	if let Some(c) = req.cookie("rl_sid") {
		return Some(c.value().to_string());
	}
	let sid = uuid::Uuid::new_v4().to_string();
	let cookie = Cookie::build(("rl_sid", sid.clone()))
		.path("/")
		.http_only(true)
		.same_site(cookie::SameSite::Lax)
		.max_age(cookie::time::Duration::days(365))
		.build();
	res.insert_cookie(cookie);
	Some(sid)
}

pub async fn require_user_key(req: &Request<Body>, res: &mut Response<Body>) -> Result<String, String> {
	ensure_sid(req, res).ok_or_else(|| "Local state is disabled (REDLIB_ENABLE_LOCAL_STATE=on required)".to_string())
}
