#![allow(clippy::cmp_owned)]

mod sqlite;

use std::sync::LazyLock;

pub use sqlite::{ChannelRow, MuteRule, PostCacheEntry, SavedPostRow, SqliteStore};

pub enum State {
	Disabled,
	Sqlite(SqliteStore),
}

pub static STATE: LazyLock<State> = LazyLock::new(|| {
	if crate::config::get_setting("REDLIB_ENABLE_LOCAL_STATE").as_deref() != Some("on") {
		return State::Disabled;
	}
	match SqliteStore::from_env() {
		Ok(s) => State::Sqlite(s),
		Err(e) => {
			eprintln!("Failed to init sqlite store: {e}");
			State::Disabled
		}
	}
});
