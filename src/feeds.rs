//! Custom feeds: named multireddits stored in a cookie.
//!
//! - GET /feeds — manage page (list, create, edit, delete)
//! - POST /feeds — create, update, or delete a feed
//! - GET /feed/:name — redirect to /r/sub1+sub2+...

use std::collections::HashMap;

use cookie::Cookie;
use hyper::{Body, Request, Response};
use time::{Duration, OffsetDateTime};

use crate::server::{RequestExt, ResponseExt};
use crate::utils::{parse_custom_feeds_cookie, redirect, template, CustomFeed, Preferences};
use askama::Template;

const MAX_FEEDS: usize = 30;
const MAX_SUBS_PER_FEED: usize = 100;
const COOKIE_MAX_AGE_WEEKS: i64 = 52;

fn set_custom_feeds_cookie(response: &mut Response<Body>, feeds: &[CustomFeed]) {
	let json = serde_json::to_string(feeds).unwrap_or_else(|_| "[]".to_string());
	// Cookie value should be URL-safe; JSON is fine but may need to stay under 4KB
	response.insert_cookie(
		Cookie::build(("custom_feeds", json))
			.path("/")
			.http_only(true)
			.expires(OffsetDateTime::now_utc() + Duration::weeks(COOKIE_MAX_AGE_WEEKS))
			.into(),
	);
}

/// Validate feed name: alphanumeric, spaces, hyphens, underscores only.
fn valid_feed_name(s: &str) -> bool {
	let s = s.trim();
	if s.is_empty() || s.len() > 50 {
		return false;
	}
	s.chars()
		.all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_')
}

/// Normalize subreddit name for URL: strip, lowercase, only valid chars.
fn normalize_sub(s: &str) -> Option<String> {
	let s = s.trim().to_lowercase();
	if s.is_empty() || s.len() > 21 {
		return None;
	}
	if s.chars()
		.all(|c| c.is_ascii_alphanumeric() || c == '_')
	{
		Some(s)
	} else {
		None
	}
}

/// Parse "sub1, sub2  sub3" or "sub1+sub2" into a sorted, deduplicated list.
fn parse_subreddits_list(input: &str) -> Vec<String> {
	let mut subs: Vec<String> = input
		.split(|c| c == ',' || c == '+' || c == ' ' || c == '\n')
		.filter_map(|s| normalize_sub(s))
		.collect();
	subs.sort();
	subs.dedup();
	subs
}

/// Feed with a URL-ready link (name percent-encoded).
pub struct FeedWithLink {
	pub feed: CustomFeed,
	pub link: String,
}

#[derive(Template)]
#[template(path = "feeds.html")]
struct FeedsTemplate {
	prefs: Preferences,
	feeds: Vec<FeedWithLink>,
	error: String,
	url: String,
}

fn feed_link(name: &str) -> String {
	format!(
		"/feed/{}",
		percent_encoding::utf8_percent_encode(name, percent_encoding::NON_ALPHANUMERIC)
	)
}

/// GET /feeds — show manage page.
pub async fn get(req: Request<Body>) -> Result<Response<Body>, String> {
	let feeds = parse_custom_feeds_cookie(&req);
	let feeds_with_links: Vec<FeedWithLink> = feeds
		.into_iter()
		.map(|f| FeedWithLink {
			link: feed_link(&f.name),
			feed: f,
		})
		.collect();
	let url = req.uri().to_string();
	Ok(template(&FeedsTemplate {
		prefs: Preferences::new(&req),
		feeds: feeds_with_links,
		error: String::new(),
		url,
	}))
}

/// POST /feeds — create, update, or delete a feed.
pub async fn post(req: Request<Body>) -> Result<Response<Body>, String> {
	let prefs = Preferences::new(&req);
	let mut feeds = parse_custom_feeds_cookie(&req);
	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes)
		.map(|(k, v)| (k.into_owned(), v.into_owned()))
		.collect();

	let action = form.get("action").map(|s| s.as_str()).unwrap_or("save");

	let mut error = String::new();

	if action == "delete" {
		let name = form.get("name").map(|s| s.trim().to_string()).unwrap_or_default();
		feeds.retain(|f| f.name != name);
	} else {
		// save or create
		let name = form.get("name").map(|s| s.trim().to_string()).unwrap_or_default();
		let subreddits_raw = form.get("subreddits").map(|s| s.as_str()).unwrap_or("");

		if !valid_feed_name(&name) {
			error = "Feed name must be 1–50 characters (letters, numbers, spaces, hyphens, underscores).".to_string();
		} else {
			let subs = parse_subreddits_list(subreddits_raw);
			if subs.is_empty() {
				error = "Add at least one subreddit.".to_string();
			} else if subs.len() > MAX_SUBS_PER_FEED {
				error = format!("Maximum {} subreddits per feed.", MAX_SUBS_PER_FEED);
			} else {
				let subreddits = subs.join("+");
				if feeds.len() >= MAX_FEEDS && !feeds.iter().any(|f| f.name == name) {
					error = format!("Maximum {} custom feeds.", MAX_FEEDS);
				} else {
					feeds.retain(|f| f.name != name);
					feeds.push(CustomFeed {
						name: name.clone(),
						subreddits,
						link: feed_link(&name),
					});
					// Keep a stable order
					feeds.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
				}
			}
		}
	}

	let url = "/feeds".to_string();
	let feeds_with_links: Vec<FeedWithLink> = feeds
		.iter()
		.map(|f| FeedWithLink {
			link: feed_link(&f.name),
			feed: f.clone(),
		})
		.collect();

	let mut res = if error.is_empty() {
		redirect("/feeds")
	} else {
		template(&FeedsTemplate {
			prefs,
			feeds: feeds_with_links,
			error,
			url,
		})
	};

	set_custom_feeds_cookie(&mut res, &feeds);
	Ok(res)
}

/// GET /feed/:name — redirect to /r/sub1+sub2+...
pub async fn redirect_to_feed(req: Request<Body>) -> Result<Response<Body>, String> {
	let name_encoded = req.param("name").unwrap_or_default();
	let name = percent_encoding::percent_decode_str(name_encoded.as_str())
		.decode_utf8()
		.unwrap_or_default()
		.to_string();
	let feeds = parse_custom_feeds_cookie(&req);
	let feed = feeds.iter().find(|f| f.name.eq_ignore_ascii_case(&name));
	match feed {
		Some(f) => {
			let path = format!("/r/{}", f.subreddits);
			Ok(redirect(&path))
		}
		None => crate::utils::error(req, "Custom feed not found").await,
	}
}
