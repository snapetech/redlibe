use super::channel::ChannelRule;
use super::cluster;
use super::presets::{preset, Lens};
use super::rank::now_ts;
use super::session::ensure_sid;
use super::view::build_fetch_subs;
use crate::server::RequestExt;
use crate::utils::{info, Post, Preferences};
use askama::Template;
use hyper::{Body, Request, Response};
use serde::Deserialize;

use crate::state::{State, STATE};

#[derive(Debug, Default, Deserialize)]
struct ClusterQuery {
	channel: Option<String>,
	lens: Option<String>,
	preset: Option<String>,
}

#[derive(Template)]
#[template(path = "cluster.html")]
struct ClusterTemplate {
	prefs: Preferences,
	channel_slug: String,
	lens: String,
	preset: String,
	cluster_id: String,
	posts: Vec<Post>,
	url: String,
}

pub async fn cluster_view(req: Request<Body>) -> Result<Response<Body>, String> {
	let cluster_id = req.param("id").unwrap_or_default();
	let prefs = Preferences::new(&req);
	let url = req.uri().to_string();

	let q: ClusterQuery = serde_urlencoded::from_str(req.uri().query().unwrap_or("")).unwrap_or_default();
	let channel_slug = q.channel.unwrap_or_else(|| "my-subs".to_string());
	let lens_str = q.lens.clone().unwrap_or_else(|| "reader".to_string());
	let preset_str = q.preset.clone().unwrap_or_default();
	let lens = Lens::parse(&lens_str);
	let preset_obj = preset(lens, &preset_str);

	let mut res = hyper::Response::new(Body::empty());
	let user_key = ensure_sid(&req, &mut res);

	// Load channel rule
	let rule = if channel_slug == "my-subs" {
		let mut r = ChannelRule::default();
		r.sources.subscriptions = true;
		r
	} else {
		let key = match &user_key {
			Some(k) => k.clone(),
			None => return info(req, "Enable local state to view cluster for custom channels.").await,
		};
		if let State::Sqlite(store) = &*STATE {
			match store.get_channel(&key, &channel_slug).await? {
				Some(row) => serde_json::from_str(&row.rule_json).unwrap_or_default(),
				None => return info(req, &format!("Channel '{}' not found.", channel_slug)).await,
			}
		} else {
			return info(req, "Local state not available.").await;
		}
	};

	// Build fetch path
	let subs = match build_fetch_subs(&rule, &prefs) {
		Ok(s) => s,
		Err(msg) => return info(req, &msg).await,
	};
	let mut path = format!("/r/{}/{sort}.json?limit=200&raw_json=1", subs.replace('+', "%2B"), sort = preset_obj.upstream_sort);
	if let Some(t) = preset_obj.upstream_t {
		path.push_str(&format!("&t={t}"));
	}

	let (posts, _) = Post::fetch(&path, false).await?;

	// Run clustering to find matching cluster (no gate filtering — show all similar posts)
	let title_items: Vec<(String, String)> = posts.iter().map(|p| (p.id.clone(), p.title.clone())).collect();
	let clusters = cluster::cluster_titles(&title_items, 4);

	let matching_members: Vec<String> = clusters.get(&cluster_id).map(|c| c.members.clone()).unwrap_or_default();

	// Drain matching posts from map (Post is not Clone, so we use a HashMap)
	let mut post_map: std::collections::HashMap<String, Post> = posts.into_iter().map(|p| (p.id.clone(), p)).collect();
	let cluster_posts: Vec<Post> = matching_members.iter().filter_map(|id| post_map.remove(id)).collect();

	let body = ClusterTemplate {
		prefs,
		channel_slug,
		lens: lens_str,
		preset: preset_str,
		cluster_id,
		posts: cluster_posts,
		url,
	};

	*res.body_mut() = Body::from(body.render().unwrap_or_default());
	*res.status_mut() = hyper::StatusCode::OK;
	res.headers_mut().insert("content-type", hyper::header::HeaderValue::from_static("text/html; charset=utf-8"));
	Ok(res)
}
