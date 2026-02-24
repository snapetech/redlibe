#![allow(clippy::cmp_owned)]

use crate::utils::{param, redirect, template, Preferences};
use askama::Template;
use hyper::{Body, Request, Response};

#[derive(Template)]
#[template(path = "go.html")]
struct GoTemplate {
	prefs: Preferences,
	url: String,
}

/// GET /go?r=subname → redirect to /r/subname. GET /go → show "Go to subreddit" form.
pub async fn get_go(req: Request<Body>) -> Result<Response<Body>, String> {
	let query = req.uri().query().unwrap_or_default();
	let path = format!("?{query}");
	let r_param = param(&path, "r").unwrap_or_default();
	let r = r_param.trim();
	if !r.is_empty() {
		let sanitized: String = r
			.chars()
			.map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
			.collect();
		let sub = sanitized.trim_matches('_');
		if !sub.is_empty() && sub.len() <= 21 {
			return Ok(redirect(&format!("/r/{sub}")));
		}
	}
	let url = req.uri().to_string();
	Ok(template(&GoTemplate {
		prefs: Preferences::new(&req),
		url,
	}))
}
