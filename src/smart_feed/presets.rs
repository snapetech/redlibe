use super::rank::{ScoreCtx, ScoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lens {
	Reader,
	Research,
	Discussion,
}

impl Lens {
	pub fn parse(s: &str) -> Self {
		match s {
			"research" => Self::Research,
			"discussion" => Self::Discussion,
			_ => Self::Reader,
		}
	}
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Reader => "reader",
			Self::Research => "research",
			Self::Discussion => "discussion",
		}
	}
}

#[derive(Debug, Clone)]
pub struct Preset {
	pub slug: &'static str,
	pub title: &'static str,
	pub lens: Lens,
	pub upstream_sort: &'static str,            // "new"|"hot"|"top"|...
	pub upstream_t: Option<&'static str>,        // only for top/controversial
	pub score: fn(&ScoreCtx) -> ScoreResult,
	pub gate: fn(&ScoreCtx) -> bool,
}

pub fn preset(lens: Lens, slug: &str) -> Preset {
	match (lens, slug) {
		(Lens::Reader, "catchup") => reader_catchup(),
		(Lens::Reader, "lownoise_text") => reader_lownoise_text(),
		(Lens::Research, "deepdives") => research_deepdives(),
		(Lens::Research, "linktracker") => research_linktracker(),
		(Lens::Discussion, "fresh_proven") => discussion_fresh_proven(),
		(Lens::Discussion, "live") => discussion_live(),
		_ => reader_digest15(),
	}
}

pub fn default_preset(lens: Lens) -> Preset {
	match lens {
		Lens::Reader => reader_digest15(),
		Lens::Research => research_deepdives(),
		Lens::Discussion => discussion_live(),
	}
}

// Reader
fn reader_digest15() -> Preset {
	Preset {
		slug: "digest15",
		title: "Digest (15m)",
		lens: Lens::Reader,
		upstream_sort: "new",
		upstream_t: None,
		gate: |ctx| ctx.num_comments >= 1 || ctx.is_pinned_source,
		score: super::rank::score_reader_digest15,
	}
}

fn reader_catchup() -> Preset {
	Preset {
		slug: "catchup",
		title: "Catch-up",
		lens: Lens::Reader,
		upstream_sort: "new",
		upstream_t: None,
		gate: |_| true,
		score: super::rank::score_reader_catchup,
	}
}

fn reader_lownoise_text() -> Preset {
	Preset {
		slug: "lownoise_text",
		title: "Low-noise (text)",
		lens: Lens::Reader,
		upstream_sort: "new",
		upstream_t: None,
		gate: |_| true,
		score: super::rank::score_reader_lownoise_text,
	}
}

// Research
fn research_deepdives() -> Preset {
	Preset {
		slug: "deepdives",
		title: "Deep dives",
		lens: Lens::Research,
		upstream_sort: "top",
		upstream_t: Some("day"),
		gate: |ctx| ctx.num_comments >= 20 || ctx.selftext_len >= 800,
		score: super::rank::score_research_deepdives,
	}
}

fn research_linktracker() -> Preset {
	Preset {
		slug: "linktracker",
		title: "Link tracker",
		lens: Lens::Research,
		upstream_sort: "new",
		upstream_t: None,
		gate: |ctx| ctx.domain_is_priority,
		score: super::rank::score_research_linktracker,
	}
}

// Discussion
fn discussion_live() -> Preset {
	Preset {
		slug: "live",
		title: "Live threads",
		lens: Lens::Discussion,
		upstream_sort: "new",
		upstream_t: None,
		gate: |ctx| ctx.num_comments >= 5 && ctx.age_hours <= 12.0,
		score: super::rank::score_discussion_live,
	}
}

fn discussion_fresh_proven() -> Preset {
	Preset {
		slug: "fresh_proven",
		title: "Fresh but proven",
		lens: Lens::Discussion,
		upstream_sort: "new",
		upstream_t: None,
		gate: |ctx| ctx.num_comments >= 10 && ctx.age_hours <= 4.0,
		score: super::rank::score_discussion_fresh_proven,
	}
}
