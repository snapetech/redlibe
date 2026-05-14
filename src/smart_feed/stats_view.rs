use super::session::{ensure_sid, local_state_enabled};
use crate::state::{ReadingStats, State, STATE};
use crate::utils::{info, Preferences};
use askama::Template;
use hyper::{Body, Request, Response};

#[derive(Template)]
#[template(path = "stats.html")]
struct StatsTemplate {
	prefs: Preferences,
	stats: ReadingStats,
	url: String,
}

pub async fn stats_view(req: Request<Body>) -> Result<Response<Body>, String> {
	if !local_state_enabled() {
		return info(req, "Enable local state (REDLIB_ENABLE_LOCAL_STATE=on) to view reading stats.").await;
	}

	let prefs = Preferences::new(&req);
	let url = req.uri().to_string();
	let mut res = Response::new(Body::empty());
	let user_key = ensure_sid(&req, &mut res);

	let stats = if let (Some(ref key), State::Sqlite(store)) = (&user_key, &*STATE) {
		store.get_stats(key).await.unwrap_or_default()
	} else {
		ReadingStats::default()
	};

	let body = StatsTemplate { prefs, stats, url };
	*res.body_mut() = Body::from(body.render().unwrap_or_default());
	*res.status_mut() = hyper::StatusCode::OK;
	res
		.headers_mut()
		.insert("content-type", hyper::header::HeaderValue::from_static("text/html; charset=utf-8"));
	Ok(res)
}
