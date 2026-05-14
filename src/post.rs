#![allow(clippy::cmp_owned)]

// CRATES
use crate::client::json;
use crate::config::get_setting;
use crate::server::{RequestExt, ResponseExt};
use crate::subreddit::{can_access_quarantine, quarantine};
use crate::utils::{
	error, format_num, get_collapsed_comment_ids, get_filters, nsfw_landing, param, parse_post, rewrite_emotes, setting, template, time, val, Author, Awards, Comment, Flair,
	FlairPart, Post, Preferences,
};
use askama::Template;
use cookie::Cookie;
use hyper::{Body, Request, Response};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use time::{Duration, OffsetDateTime};

// STRUCTS
#[derive(Template)]
#[template(path = "post.html")]
struct PostTemplate {
	comments: Vec<Comment>,
	post: Post,
	sort: String,
	prefs: Preferences,
	single_thread: bool,
	url: String,
	url_without_query: String,
	comment_query: String,
	reader_mode: bool,
}

static COMMENT_SEARCH_CAPTURE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\?q=(.*)&type=comment").unwrap());

pub async fn item(req: Request<Body>) -> Result<Response<Body>, String> {
	// Build Reddit API path
	let mut path: String = format!("{}.json?{}&raw_json=1", req.uri().path(), req.uri().query().unwrap_or_default());
	let sub = req.param("sub").unwrap_or_default();
	let quarantined = can_access_quarantine(&req, &sub);
	let url = req.uri().to_string();

	// Set sort to sort query parameter
	let sort = param(&path, "sort").unwrap_or_else(|| {
		// Grab default comment sort method from Cookies
		let default_sort = setting(&req, "comment_sort");

		// If there's no sort query but there's a default sort, set sort to default_sort
		if default_sort.is_empty() {
			String::new()
		} else {
			path = format!("{}.json?{}&sort={}&raw_json=1", req.uri().path(), req.uri().query().unwrap_or_default(), default_sort);
			default_sort
		}
	});

	// Log the post ID being fetched in debug mode
	#[cfg(debug_assertions)]
	req.param("id").unwrap_or_default();

	let single_thread = req.param("comment_id").is_some();
	let highlighted_comment = &req.param("comment_id").unwrap_or_default();

	// Send a request to the url, receive JSON in response
	match json(path, quarantined).await {
		// Otherwise, grab the JSON output from the request
		Ok(response) => {
			// Parse the JSON into Post and Comment structs
			let post = parse_post(&response[0]["data"]["children"][0]).await;

			let req_url = req.uri().to_string();
			// Return landing page if this post if this Reddit deems this post
			// NSFW, but we have also disabled the display of NSFW content
			// or if the instance is SFW-only.
			if post.nsfw && crate::utils::should_be_nsfw_gated(&req, &req_url) {
				return Ok(nsfw_landing(req, req_url).await.unwrap_or_default());
			}

			let query_body = match COMMENT_SEARCH_CAPTURE.captures(&url) {
				Some(captures) => captures.get(1).unwrap().as_str().replace("%20", " ").replace('+', " "),
				None => String::new(),
			};

			let query_string = format!("q={query_body}&type=comment");
			let form = url::form_urlencoded::parse(query_string.as_bytes()).collect::<HashMap<_, _>>();
			let query = form.get("q").unwrap().clone().to_string();

			let collapsed_ids = get_collapsed_comment_ids(&req);
			let comments = match query.as_str() {
				"" => parse_comments(
					&response[1],
					&post.permalink,
					&post.author.name,
					highlighted_comment,
					&get_filters(&req),
					&collapsed_ids,
					&req,
				),
				_ => query_comments(
					&response[1],
					&post.permalink,
					&post.author.name,
					highlighted_comment,
					&get_filters(&req),
					&query,
					&collapsed_ids,
					&req,
				),
			};

			let path_for_param = format!("?{}", req.uri().query().unwrap_or_default());
			let reader_mode = param(&path_for_param, "reader").map(|s| s == "1" || s == "on").unwrap_or(false);

			// Use the Post and Comment structs to generate a website to show users
			Ok(template(&PostTemplate {
				comments,
				post,
				url_without_query: url.clone().trim_end_matches(&format!("?q={query}&type=comment")).to_string(),
				sort,
				prefs: Preferences::new(&req),
				single_thread,
				url: req_url,
				comment_query: query,
				reader_mode,
			}))
		}
		// If the Reddit API returns an error, exit and send error page to user
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

/// POST /comment-collapse: body id=t1_xxx&action=collapse|expand — persist collapsed comment state in cookie.
pub async fn comment_collapse(req: Request<Body>) -> Result<Response<Body>, String> {
	let existing = get_collapsed_comment_ids(&req);
	let body_bytes = hyper::body::to_bytes(req.into_body()).await.map_err(|e| e.to_string())?;
	let form: std::collections::HashMap<String, String> = url::form_urlencoded::parse(&body_bytes).map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
	let id = form.get("id").map(|s| s.trim()).unwrap_or("");
	let action = form.get("action").map(|s| s.as_str()).unwrap_or("");
	if id.is_empty() || !id.starts_with("t1_") || !matches!(action, "collapse" | "expand") {
		let res = Response::builder().status(400).body(Body::empty()).unwrap_or_default();
		return Ok(res);
	}
	let mut ids: Vec<String> = existing.into_iter().collect();
	match action {
		"collapse" => {
			if !ids.contains(&id.to_string()) {
				ids.push(id.to_string());
			}
		}
		"expand" => ids.retain(|x| x != id),
		_ => {}
	}
	ids.sort();
	let value = ids.join(",");
	let mut response = Response::builder().status(204).body(Body::empty()).unwrap_or_default();
	response.insert_cookie(
		Cookie::build(("collapsed_comment_ids", value))
			.path("/")
			.http_only(true)
			.expires(OffsetDateTime::now_utc() + Duration::weeks(52))
			.into(),
	);
	Ok(response)
}

// COMMENTS

fn parse_comments(
	json: &serde_json::Value,
	post_link: &str,
	post_author: &str,
	highlighted_comment: &str,
	filters: &HashSet<String>,
	collapsed_ids: &HashSet<String>,
	req: &Request<Body>,
) -> Vec<Comment> {
	// Parse the comment JSON into a Vector of Comments
	let comments = json["data"]["children"].as_array().map_or(Vec::new(), std::borrow::ToOwned::to_owned);

	// For each comment, retrieve the values to build a Comment object
	comments
		.into_iter()
		.map(|comment| {
			let data = &comment["data"];
			let replies: Vec<Comment> = if data["replies"].is_object() {
				parse_comments(&data["replies"], post_link, post_author, highlighted_comment, filters, collapsed_ids, req)
			} else {
				Vec::new()
			};
			build_comment(&comment, data, replies, post_link, post_author, highlighted_comment, filters, collapsed_ids, req)
		})
		.collect()
}

fn query_comments(
	json: &serde_json::Value,
	post_link: &str,
	post_author: &str,
	highlighted_comment: &str,
	filters: &HashSet<String>,
	query: &str,
	collapsed_ids: &HashSet<String>,
	req: &Request<Body>,
) -> Vec<Comment> {
	let comments = json["data"]["children"].as_array().map_or(Vec::new(), std::borrow::ToOwned::to_owned);
	let mut results = Vec::new();

	for comment in comments {
		let data = &comment["data"];

		// If this comment contains replies, handle those too
		if data["replies"].is_object() {
			results.append(&mut query_comments(
				&data["replies"],
				post_link,
				post_author,
				highlighted_comment,
				filters,
				query,
				collapsed_ids,
				req,
			));
		}

		let c = build_comment(&comment, data, Vec::new(), post_link, post_author, highlighted_comment, filters, collapsed_ids, req);
		if c.body.to_lowercase().contains(&query.to_lowercase()) {
			results.push(c);
		}
	}

	results
}
#[allow(clippy::too_many_arguments)]
fn build_comment(
	comment: &serde_json::Value,
	data: &serde_json::Value,
	replies: Vec<Comment>,
	post_link: &str,
	post_author: &str,
	highlighted_comment: &str,
	filters: &HashSet<String>,
	collapsed_ids: &HashSet<String>,
	req: &Request<Body>,
) -> Comment {
	let id = val(comment, "id");

	let body = if (val(comment, "author") == "[deleted]" && val(comment, "body") == "[removed]") || val(comment, "body") == "[ Removed by Reddit ]" {
		format!(
			"<div class=\"md\"><p>[removed] — <a href=\"https://{}{post_link}{id}\">view removed comment</a></p></div>",
			get_setting("REDLIB_PUSHSHIFT_FRONTEND").unwrap_or_else(|| String::from(crate::config::DEFAULT_PUSHSHIFT_FRONTEND)),
		)
	} else {
		rewrite_emotes(&data["media_metadata"], val(comment, "body_html"))
	};
	let kind = comment["kind"].as_str().unwrap_or_default().to_string();

	let unix_time = data["created_utc"].as_f64().unwrap_or_default();
	let (rel_time, created) = time(unix_time);

	let edited = data["edited"].as_f64().map_or((String::new(), String::new()), time);

	let score = data["score"].as_i64().unwrap_or(0);

	// The JSON API only provides comments up to some threshold.
	// Further comments have to be loaded by subsequent requests.
	// The "kind" value will be "more" and the "count"
	// shows how many more (sub-)comments exist in the respective nesting level.
	// Note that in certain (seemingly random) cases, the count is simply wrong.
	let more_count = data["count"].as_i64().unwrap_or_default();

	let awards: Awards = Awards::parse(&data["all_awardings"]);

	let parent_kind_and_id = val(comment, "parent_id");
	let parent_info = parent_kind_and_id.split('_').collect::<Vec<&str>>();

	let highlighted = id == highlighted_comment;

	let author = Author {
		name: val(comment, "author"),
		flair: Flair {
			flair_parts: FlairPart::parse(
				data["author_flair_type"].as_str().unwrap_or_default(),
				data["author_flair_richtext"].as_array(),
				data["author_flair_text"].as_str(),
			),
			text: val(comment, "link_flair_text"),
			background_color: val(comment, "author_flair_background_color"),
			foreground_color: val(comment, "author_flair_text_color"),
		},
		distinguished: val(comment, "distinguished"),
	};
	let is_filtered = filters.contains(&["u_", author.name.as_str()].concat());

	// Many subreddits have a default comment posted about the sub's rules etc.
	// Many Redlib users do not wish to see this kind of comment by default.
	// Reddit does not tell us which users are "bots", so a good heuristic is to
	// collapse stickied moderator comments.
	let is_moderator_comment = data["distinguished"].as_str().unwrap_or_default() == "moderator";
	let is_stickied = data["stickied"].as_bool().unwrap_or_default();
	let user_collapsed = collapsed_ids.contains(&format!("t1_{id}"));
	let collapsed = (is_moderator_comment && is_stickied) || is_filtered || user_collapsed;

	Comment {
		id,
		kind,
		parent_id: parent_info[1].to_string(),
		parent_kind: parent_info[0].to_string(),
		post_link: post_link.to_string(),
		post_author: post_author.to_string(),
		body,
		author,
		score: if data["score_hidden"].as_bool().unwrap_or_default() {
			("\u{2022}".to_string(), "Hidden".to_string())
		} else {
			format_num(score)
		},
		rel_time,
		created,
		edited,
		replies,
		highlighted,
		awards,
		collapsed,
		is_filtered,
		more_count,
		prefs: Preferences::new(req),
	}
}
