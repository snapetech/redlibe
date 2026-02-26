use crate::server::RequestExt;
use base64::Engine;
use cookie::Cookie;
use hyper::{Body, Request, Response};

const CSRF_COOKIE: &str = "rl_csrf";

fn rand_token() -> String {
	// 32 bytes random, base64url
	let mut bytes = [0u8; 32];
	for b in bytes.iter_mut() {
		*b = fastrand::u8(..);
	}
	base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn ensure_csrf_cookie(req: &Request<Body>, res: &mut Response<Body>) -> String {
	use crate::server::ResponseExt;
	if let Some(c) = req.cookie(CSRF_COOKIE) {
		return c.value().to_string();
	}
	let token = rand_token();
	let cookie = Cookie::build((CSRF_COOKIE, token.clone()))
		.path("/")
		.http_only(true)
		.same_site(cookie::SameSite::Lax)
		.max_age(cookie::time::Duration::days(90))
		.finish();
	res.insert_cookie(cookie);
	token
}

pub fn verify_csrf(req: &Request<Body>, form_token: &str) -> Result<(), String> {
	let Some(c) = req.cookie(CSRF_COOKIE) else {
		return Err("Missing CSRF cookie".to_string());
	};
	if c.value() != form_token {
		return Err("CSRF token mismatch".to_string());
	}
	Ok(())
}
