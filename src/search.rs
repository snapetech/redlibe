#![allow(clippy::cmp_owned)]

// CRATES
use crate::utils::{
	self, catch_random, error, filter_media_only, filter_posts, filter_posts_by_content, filter_read_posts, format_num, format_url, get_filter_domains, get_filter_flairs,
	get_filter_keywords, get_filters, get_read_ids, get_recent_searches, get_saved_searches, param, recent_searches_cookie_value, redirect,
	saved_searches_cookie_value_after_save, saved_searches_cookie_value_after_unsave, setting, template, val, Post, Preferences, SavedSearch,
};
use crate::{
	client::json,
	server::{RequestExt, ResponseExt},
	subreddit::{can_access_quarantine, quarantine},
};
use askama::Template;
use cookie::Cookie;
use hyper::{Body, Request, Response};
use percent_encoding::utf8_percent_encode;
use percent_encoding::NON_ALPHANUMERIC;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use time::{Duration, OffsetDateTime};

// STRUCTS
#[derive(Clone)]
pub struct RecentSearch {
	pub query: String,
	pub url: String,
}

struct SearchParams {
	q: String,
	sort: String,
	t: String,
	before: String,
	after: String,
	restrict_sr: String,
	typed: String,
}

// STRUCTS
struct Subreddit {
	name: String,
	url: String,
	icon: String,
	description: String,
	subscribers: (String, String),
}

#[derive(Template)]
#[template(path = "search.html")]
struct SearchTemplate {
	posts: Vec<Post>,
	subreddits: Vec<Subreddit>,
	sub: String,
	params: SearchParams,
	prefs: Preferences,
	url: String,
	/// Whether the subreddit itself is filtered.
	is_filtered: bool,
	/// Whether all fetched posts are filtered (to differentiate between no posts fetched in the first place,
	/// and all fetched posts being filtered).
	all_posts_filtered: bool,
	/// Whether all posts were hidden because they are NSFW (and user has disabled show NSFW)
	all_posts_hidden_nsfw: bool,
	no_posts: bool,
	recent_searches: Vec<RecentSearch>,
	saved_searches: Vec<SavedSearch>,
}

/// Regex matched against search queries to determine if they are reddit urls.
static REDDIT_URL_MATCH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^https?://([^\./]+\.)*reddit.com/").unwrap());

// SERVICES
pub async fn find(req: Request<Body>) -> Result<Response<Body>, String> {
	// This ensures that during a search, no NSFW posts are fetched at all
	let nsfw_results = if setting(&req, "show_nsfw") == "on" && !utils::sfw_only() {
		"&include_over_18=on"
	} else {
		""
	};
	let uri_path = req.uri().path().replace("+", "%2B");
	let path = format!("{}.json?{}{}&raw_json=1", uri_path, req.uri().query().unwrap_or_default(), nsfw_results);
	let mut query = param(&path, "q").unwrap_or_default();
	query = REDDIT_URL_MATCH.replace(&query, "").to_string();

	if query.is_empty() {
		return Ok(redirect("/"));
	}

	if query.starts_with("r/") || query.starts_with("user/") {
		return Ok(redirect(&format!("/{query}")));
	}

	if query.starts_with("R/") {
		return Ok(redirect(&format!("/r{}", &query[1..])));
	}

	if query.starts_with("u/") || query.starts_with("U/") {
		return Ok(redirect(&format!("/user{}", &query[1..])));
	}

	let sub = req.param("sub").unwrap_or_default();
	let quarantined = can_access_quarantine(&req, &sub);
	// Handle random subreddits
	if let Ok(random) = catch_random(&sub, "/find").await {
		return Ok(random);
	}

	let typed = param(&path, "type").unwrap_or_default();

	let sort = param(&path, "sort").unwrap_or_else(|| "relevance".to_string());
	let filters = get_filters(&req);
	let recent_searches: Vec<RecentSearch> = get_recent_searches(&req)
		.into_iter()
		.map(|q| {
			let url = format!("/search?q={}", utf8_percent_encode(&q, NON_ALPHANUMERIC));
			RecentSearch { query: q, url }
		})
		.collect();

	// If search is not restricted to this subreddit, show other subreddits in search results
	let subreddits = if param(&path, "restrict_sr").is_none() {
		let mut subreddits = search_subreddits(&query, &typed).await;
		subreddits.retain(|s| !filters.contains(s.name.as_str()));
		subreddits
	} else {
		Vec::new()
	};

	let url = String::from(req.uri().path_and_query().map_or("", |val| val.as_str()));

	let saved_searches = get_saved_searches(&req);

	// If all requested subs are filtered, we don't need to fetch posts.
	if sub.split('+').all(|s| filters.contains(s)) {
		Ok(template(&SearchTemplate {
			posts: Vec::new(),
			subreddits,
			sub,
			params: SearchParams {
				q: query.replace('"', "&quot;"),
				sort,
				t: param(&path, "t").unwrap_or_default(),
				before: param(&path, "after").unwrap_or_default(),
				after: String::new(),
				restrict_sr: param(&path, "restrict_sr").unwrap_or_default(),
				typed,
			},
			prefs: Preferences::new(&req),
			url,
			is_filtered: true,
			all_posts_filtered: false,
			all_posts_hidden_nsfw: false,
			no_posts: false,
			recent_searches: recent_searches.clone(),
			saved_searches: saved_searches.clone(),
		}))
	} else {
		match Post::fetch(&path, quarantined).await {
			Ok((mut posts, after)) => {
				let (_, all_posts_filtered) = filter_posts(&mut posts, &filters);
				filter_posts_by_content(&mut posts, &get_filter_keywords(&req), &get_filter_flairs(&req), &get_filter_domains(&req));
				if setting(&req, "hide_read") == "on" {
					let read_ids = get_read_ids(&req);
					filter_read_posts(&mut posts, &read_ids);
				}
				if setting(&req, "show_only_media") == "on" {
					filter_media_only(&mut posts);
				}
				let no_posts = posts.is_empty();
				let all_posts_hidden_nsfw = !no_posts && (posts.iter().all(|p| p.flags.nsfw) && setting(&req, "show_nsfw") != "on");
				let mut res = template(&SearchTemplate {
					posts,
					subreddits,
					sub,
					params: SearchParams {
						q: query.replace('"', "&quot;"),
						sort,
						t: param(&path, "t").unwrap_or_default(),
						before: param(&path, "after").unwrap_or_default(),
						after,
						restrict_sr: param(&path, "restrict_sr").unwrap_or_default(),
						typed,
					},
					prefs: Preferences::new(&req),
					url,
					is_filtered: false,
					all_posts_filtered,
					all_posts_hidden_nsfw,
					no_posts,
					recent_searches: recent_searches.clone(),
					saved_searches: saved_searches.clone(),
				});
				if !query.is_empty() {
					let val = recent_searches_cookie_value(&req, &query);
					if !val.is_empty() {
						res.insert_cookie(
							Cookie::build(("recent_searches", val))
								.path("/")
								.http_only(true)
								.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
								.into(),
						);
					}
				}
				Ok(res)
			}
			Err(msg) => {
				if msg == "quarantined" || msg == "gated" {
					let sub = req.param("sub").unwrap_or_default();
					Ok(quarantine(&req, sub, &msg))
				} else {
					error(req, &msg).await
				}
			}
		}
	}
}

async fn search_subreddits(q: &str, typed: &str) -> Vec<Subreddit> {
	let limit = if typed == "sr_user" { "50" } else { "3" };
	let subreddit_search_path = format!("/subreddits/search.json?q={}&limit={limit}", q.replace(' ', "+"));

	// Send a request to the url
	json(subreddit_search_path, false).await.unwrap_or_default()["data"]["children"]
		.as_array()
		.map(ToOwned::to_owned)
		.unwrap_or_default()
		.iter()
		.map(|subreddit| {
			// For each subreddit from subreddit list
			// Fetch subreddit icon either from the community_icon or icon_img value
			let icon = subreddit["data"]["community_icon"].as_str().map_or_else(|| val(subreddit, "icon_img"), ToString::to_string);

			Subreddit {
				name: val(subreddit, "display_name"),
				url: val(subreddit, "url"),
				icon: format_url(&icon),
				description: val(subreddit, "public_description"),
				subscribers: format_num(subreddit["data"]["subscribers"].as_f64().unwrap_or_default() as i64),
			}
		})
		.collect::<Vec<Subreddit>>()
}

/// POST /search/save — add current search to saved list (body: q=... & name=optional).
pub async fn save(req: Request<Body>) -> Result<Response<Body>, String> {
	let (parts, body) = req.into_parts();
	let body_bytes = hyper::body::to_bytes(body).await.map_err(|e| e.to_string())?;
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
	let q = form.get("q").cloned().unwrap_or_default();
	let name = form.get("name").cloned().unwrap_or_default();
	let req2 = Request::from_parts(parts, Body::empty());
	let cookie_val = saved_searches_cookie_value_after_save(&req2, &name, &q);
	let redirect_url = if q.is_empty() {
		"/search".to_string()
	} else {
		format!("/search?q={}", utf8_percent_encode(&q, NON_ALPHANUMERIC))
	};
	let mut res = redirect(&redirect_url);
	res.insert_cookie(
		Cookie::build(("saved_searches", cookie_val))
			.path("/")
			.http_only(true)
			.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
			.into(),
	);
	Ok(res)
}

/// POST /search/unsave — remove a search by query (body: q=...).
pub async fn unsave(req: Request<Body>) -> Result<Response<Body>, String> {
	let (parts, body) = req.into_parts();
	let body_bytes = hyper::body::to_bytes(body).await.map_err(|e| e.to_string())?;
	let form: HashMap<String, String> = url::form_urlencoded::parse(&body_bytes).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
	let q = form.get("q").cloned().unwrap_or_default();
	let req2 = Request::from_parts(parts, Body::empty());
	let cookie_val = saved_searches_cookie_value_after_unsave(&req2, &q);
	let redirect_url = if q.is_empty() {
		"/search".to_string()
	} else {
		format!("/search?q={}", utf8_percent_encode(&q, NON_ALPHANUMERIC))
	};
	let mut res = redirect(&redirect_url);
	res.insert_cookie(
		Cookie::build(("saved_searches", cookie_val))
			.path("/")
			.http_only(true)
			.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
			.into(),
	);
	Ok(res)
}
