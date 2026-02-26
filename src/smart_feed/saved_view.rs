use super::csrf;
use super::session::{ensure_sid, local_state_enabled};
use crate::state::{State, STATE};
use crate::utils::{info, time, Preferences};
use askama::Template;
use hyper::{Body, Request, Response};

pub struct SavedItem {
	pub post_id: String,
	pub title: String,
	pub community: String,
	pub domain: String,
	pub permalink: String,
	pub score: i64,
	pub comments: i64,
	pub rel_time: String,
	pub saved_at_display: String,
}

#[derive(Template)]
#[template(path = "saved.html")]
struct SavedTemplate {
	prefs: Preferences,
	items: Vec<SavedItem>,
	csrf: String,
	url: String,
}

pub async fn saved_view(req: Request<Body>) -> Result<Response<Body>, String> {
	if !local_state_enabled() {
		return info(req, "Enable local state (REDLIB_ENABLE_LOCAL_STATE=on) to use saved posts.").await;
	}

	let prefs = Preferences::new(&req);
	let url = req.uri().to_string();
	let mut res = Response::new(Body::empty());

	let user_key = ensure_sid(&req, &mut res);
	let csrf_tok = csrf::ensure_csrf_cookie(&req, &mut res);

	let rows = if let Some(ref key) = user_key {
		if let State::Sqlite(store) = &*STATE {
			store.get_saved_posts(key).await.unwrap_or_default()
		} else {
			Vec::new()
		}
	} else {
		Vec::new()
	};

	let items: Vec<SavedItem> = rows
		.into_iter()
		.map(|r| {
			let (rel_time, _) = time(r.created_utc as f64);
			let (saved_display, _) = time(r.saved_at as f64);
			SavedItem {
				post_id: r.post_id,
				title: r.title,
				community: r.community,
				domain: r.domain,
				permalink: r.permalink,
				score: r.score,
				comments: r.comments,
				rel_time,
				saved_at_display: saved_display,
			}
		})
		.collect();

	let body = SavedTemplate { prefs, items, csrf: csrf_tok, url };
	*res.body_mut() = Body::from(body.render().unwrap_or_default());
	*res.status_mut() = hyper::StatusCode::OK;
	res.headers_mut().insert("content-type", hyper::header::HeaderValue::from_static("text/html; charset=utf-8"));
	Ok(res)
}
