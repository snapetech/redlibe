use crate::utils::Post;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct ScoreCtx<'a> {
	pub post: &'a Post,
	pub now_ts: i64,
	pub age_hours: f64,
	pub num_comments: i64,
	pub score: i64,
	pub comment_velocity: f64,
	pub selftext_len: usize,
	pub is_self: bool,
	pub domain_is_priority: bool,
	pub is_pinned_source: bool,
}

#[derive(Debug, Clone)]
pub struct ScoreResult {
	pub score: f64,
	pub why: Vec<String>,
}

fn ln1p(x: f64) -> f64 {
	(1.0 + x).ln()
}

pub fn build_score_ctx<'a>(post: &'a Post, now_ts: i64, domain_is_priority: bool, is_pinned_source: bool) -> ScoreCtx<'a> {
	let age_s = (now_ts - post.created_ts as i64).max(0) as f64;
	let age_hours = age_s / 3600.0;

	let num_comments = post.comments.1.parse::<i64>().unwrap_or(0);
	let score = post.score.1.parse::<i64>().unwrap_or(0);

	let denom = age_hours.max(0.5);
	let comment_velocity = (num_comments as f64) / denom;

	let selftext_len = post.body.len();
	let is_self = post.post_type == "self";

	ScoreCtx {
		post,
		now_ts,
		age_hours,
		num_comments,
		score,
		comment_velocity,
		selftext_len,
		is_self,
		domain_is_priority,
		is_pinned_source,
	}
}

// -------- Preset scoring functions --------

// Reader: digest15
pub fn score_reader_digest15(ctx: &ScoreCtx) -> ScoreResult {
	let nc = ln1p(ctx.num_comments as f64);
	let ns = ln1p(ctx.score.max(0) as f64);
	let cv = ln1p(ctx.comment_velocity);
	let fresh = 1.0 / (1.0 + ctx.age_hours);

	let mut why = Vec::new();
	if ctx.num_comments >= 10 {
		why.push("Active discussion".into());
	}
	if ctx.comment_velocity >= 5.0 {
		why.push("High comment velocity".into());
	}
	if ctx.age_hours <= 6.0 {
		why.push("Fresh".into());
	}

	let s = 1.2 * cv + 0.6 * nc + 0.2 * ns + 0.3 * fresh;
	ScoreResult { score: s, why }
}

pub fn score_reader_catchup(ctx: &ScoreCtx) -> ScoreResult {
	let nc = ln1p(ctx.num_comments as f64);
	let ns = ln1p(ctx.score.max(0) as f64);
	let cv = ln1p(ctx.comment_velocity);
	let fresh = 1.0 / (1.0 + ctx.age_hours);

	let mut why = Vec::new();
	if ctx.num_comments >= 20 {
		why.push("Lots of comments".into());
	}

	let s = 0.8 * nc + 0.5 * cv + 0.2 * ns + 0.1 * fresh;
	ScoreResult { score: s, why }
}

pub fn score_reader_lownoise_text(ctx: &ScoreCtx) -> ScoreResult {
	let nc = ln1p(ctx.num_comments as f64);
	let cv = ln1p(ctx.comment_velocity);
	let ns = ln1p(ctx.score.max(0) as f64);

	let is_self_bonus = if ctx.is_self { 1.0 } else { -0.3 };
	let long_bonus = (ctx.selftext_len.min(2000) as f64) / 2000.0;

	let mut why = Vec::new();
	if ctx.is_self {
		why.push("Text post".into());
	}
	if ctx.selftext_len >= 800 {
		why.push("Longform".into());
	}

	let s = 1.0 * nc + 0.6 * cv + 0.4 * is_self_bonus + 0.2 * ns + 0.3 * long_bonus;
	ScoreResult { score: s, why }
}

// Research
pub fn score_research_deepdives(ctx: &ScoreCtx) -> ScoreResult {
	let nc = ln1p(ctx.num_comments as f64);
	let cv = ln1p(ctx.comment_velocity);

	// Stability: prefer 12-36h over ultra-fresh for research
	let stab = if (12.0..=36.0).contains(&ctx.age_hours) { 1.0 } else { 0.0 };

	let mut why = Vec::new();
	if ctx.num_comments >= 50 {
		why.push("High comment volume".into());
	}
	if stab > 0.0 {
		why.push("Settled thread".into());
	}

	let s = 1.4 * nc + 0.2 * cv + 0.2 * stab;
	ScoreResult { score: s, why }
}

pub fn score_research_linktracker(ctx: &ScoreCtx) -> ScoreResult {
	let cv = ln1p(ctx.comment_velocity);
	let nc = ln1p(ctx.num_comments as f64);
	let fresh = 1.0 / (1.0 + ctx.age_hours);

	let domain_priority = if ctx.domain_is_priority { 1.0 } else { 0.0 };

	let mut why = Vec::new();
	if domain_priority > 0.0 {
		why.push("Tracked domain".into());
	}

	let s = 1.0 * domain_priority + 0.8 * cv + 0.3 * nc + 0.2 * fresh;
	ScoreResult { score: s, why }
}

// Discussion
pub fn score_discussion_live(ctx: &ScoreCtx) -> ScoreResult {
	let cv = ln1p(ctx.comment_velocity);
	let nc = ln1p(ctx.num_comments as f64);
	let fresh = 1.0 / (1.0 + ctx.age_hours);

	let mut why = Vec::new();
	why.push(format!("{:.1} comments/hr", ctx.comment_velocity));

	let s = 1.8 * cv + 0.6 * nc + 0.2 * fresh;
	ScoreResult { score: s, why }
}

pub fn score_discussion_fresh_proven(ctx: &ScoreCtx) -> ScoreResult {
	let cv = ln1p(ctx.comment_velocity);
	let nc = ln1p(ctx.num_comments as f64);
	let fresh = 1.0 / (1.0 + ctx.age_hours);

	let mut why = Vec::new();
	if ctx.age_hours <= 2.0 {
		why.push("Very fresh".into());
	}
	if ctx.num_comments >= 10 {
		why.push("Already has traction".into());
	}

	let s = 1.6 * cv + 0.3 * nc + 0.4 * fresh;
	ScoreResult { score: s, why }
}

pub fn now_ts() -> i64 {
	SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}
