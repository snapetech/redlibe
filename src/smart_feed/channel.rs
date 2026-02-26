use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRule {
	pub sources: Sources,
	pub filters: Filters,
	pub gates: Gates,
	pub presentation: Presentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sources {
	pub subscriptions: bool,
	pub subreddits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filters {
	pub include_keywords: Vec<String>,
	pub exclude_keywords: Vec<String>,
	pub domains_allow: Vec<String>,
	pub domains_block: Vec<String>,
	pub media_types: Vec<String>,
	pub nsfw: String, // "show"|"blur"|"hide"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gates {
	pub min_comments: i64,
	pub min_score: i64,
	pub max_age_hours: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presentation {
	pub clusters: String, // "on"|"off"
	pub density: String,  // "comfort"|"balanced"|"dense"
}

impl Default for ChannelRule {
	fn default() -> Self {
		Self {
			sources: Sources {
				subscriptions: true,
				subreddits: Vec::new(),
			},
			filters: Filters {
				include_keywords: Vec::new(),
				exclude_keywords: Vec::new(),
				domains_allow: Vec::new(),
				domains_block: Vec::new(),
				media_types: vec!["self".into(), "link".into(), "image".into(), "video".into(), "gif".into(), "gallery".into()],
				nsfw: "hide".into(),
			},
			gates: Gates {
				min_comments: 0,
				min_score: 0,
				max_age_hours: 72,
			},
			presentation: Presentation {
				clusters: "on".into(),
				density: "balanced".into(),
			},
		}
	}
}
