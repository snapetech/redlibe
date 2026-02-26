use crate::config::get_setting;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task::spawn_blocking;

#[derive(Debug, Clone)]
pub struct MuteRule {
	pub id: i64,
	pub scope: String,
	pub rule_type: String,
	pub pattern: String,
}

#[derive(Debug, Clone)]
pub struct ChannelRow {
	pub slug: String,
	pub title: String,
	pub rule_json: String,
	pub created_at: i64,
	pub updated_at: i64,
	pub sort_order: i64,
}

#[derive(Debug, Clone)]
pub struct PostCacheEntry {
	pub post_id: String,
	pub title: String,
	pub community: String,
	pub domain: String,
	pub permalink: String,
	pub score: i64,
	pub comments: i64,
	pub created_utc: i64,
}

#[derive(Debug, Clone, Default)]
pub struct ReadingStats {
	pub total_seen: i64,
	pub total_read: i64,
	pub total_saved: i64,
	pub total_mutes: i64,
	pub total_channels: i64,
	pub top_communities: Vec<(String, i64)>,
}

#[derive(Debug, Clone)]
pub struct SavedPostRow {
	pub post_id: String,
	pub title: String,
	pub community: String,
	pub domain: String,
	pub permalink: String,
	pub score: i64,
	pub comments: i64,
	pub created_utc: i64,
	pub saved_at: i64,
}

#[derive(Clone)]
pub struct SqliteStore {
	pool: Pool<SqliteConnectionManager>,
}

fn now_ts() -> i64 {
	SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

impl SqliteStore {
	pub fn from_env() -> Result<Self, String> {
		let db_path = get_setting("REDLIB_DB_PATH").unwrap_or_else(|| "redlib.sqlite".into());
		let mgr = SqliteConnectionManager::file(db_path);
		let pool = Pool::builder().max_size(16).build(mgr).map_err(|e| e.to_string())?;

		{
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn
				.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
				.map_err(|e| e.to_string())?;
			migrate(&conn)?;
			prune_old_state(&conn)?;
		}

		Ok(Self { pool })
	}

	// ---- Mute rules ----

	pub async fn list_mutes(&self, user_key: &str, scope: &str) -> Result<Vec<MuteRule>, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let scope = scope.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			let mut stmt = conn
				.prepare("SELECT id, scope, rule_type, pattern FROM mute_rule WHERE user_key = ?1 AND scope = ?2 ORDER BY created_at DESC")
				.map_err(|e| e.to_string())?;
			let rows = stmt
				.query_map(params![user_key, scope], |row| {
					Ok(MuteRule { id: row.get(0)?, scope: row.get(1)?, rule_type: row.get(2)?, pattern: row.get(3)? })
				})
				.map_err(|e| e.to_string())?;
			let mut out = Vec::new();
			for r in rows {
				out.push(r.map_err(|e| e.to_string())?);
			}
			Ok(out)
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn list_all_mutes(&self, user_key: &str) -> Result<Vec<MuteRule>, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			let mut stmt = conn
				.prepare("SELECT id, scope, rule_type, pattern FROM mute_rule WHERE user_key = ?1 ORDER BY scope ASC, created_at DESC")
				.map_err(|e| e.to_string())?;
			let rows = stmt
				.query_map(params![user_key], |row| {
					Ok(MuteRule { id: row.get(0)?, scope: row.get(1)?, rule_type: row.get(2)?, pattern: row.get(3)? })
				})
				.map_err(|e| e.to_string())?;
			let mut out = Vec::new();
			for r in rows {
				out.push(r.map_err(|e| e.to_string())?);
			}
			Ok(out)
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn add_mute(&self, user_key: &str, scope: &str, rule_type: &str, pattern: &str) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let scope = scope.to_string();
		let rule_type = rule_type.to_string();
		let pattern = pattern.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn.execute(
				"INSERT OR IGNORE INTO mute_rule(user_key, scope, rule_type, pattern, created_at) VALUES(?1, ?2, ?3, ?4, strftime('%s','now'))",
				params![user_key, scope, rule_type, pattern],
			)
			.map_err(|e| e.to_string())?;
			Ok(())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn delete_mute(&self, user_key: &str, mute_id: i64) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn.execute("DELETE FROM mute_rule WHERE id = ?1 AND user_key = ?2", params![mute_id, user_key])
				.map_err(|e| e.to_string())?;
			Ok(())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	// ---- Post state ----

	pub async fn get_read_map(&self, user_key: &str, post_ids: &[String]) -> Result<HashMap<String, bool>, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let ids = post_ids.to_vec();
		spawn_blocking(move || {
			let mut conn = pool.get().map_err(|e| e.to_string())?;
			let mut out: HashMap<String, bool> = HashMap::new();
			let tx = conn.transaction().map_err(|e| e.to_string())?;
			{
				let mut stmt = tx
					.prepare("SELECT post_id, is_read FROM post_state WHERE user_key = ?1 AND post_id = ?2")
					.map_err(|e| e.to_string())?;
				for id in ids {
					if let Some((pid, is_read)) = stmt
						.query_row(params![user_key, id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
						.optional()
						.map_err(|e| e.to_string())?
					{
						out.insert(pid, is_read != 0);
					}
				}
			}
			tx.commit().map_err(|e| e.to_string())?;
			Ok(out)
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn get_saved_map(&self, user_key: &str, post_ids: &[String]) -> Result<HashMap<String, bool>, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let ids = post_ids.to_vec();
		spawn_blocking(move || {
			let mut conn = pool.get().map_err(|e| e.to_string())?;
			let mut out: HashMap<String, bool> = HashMap::new();
			let tx = conn.transaction().map_err(|e| e.to_string())?;
			{
				let mut stmt = tx
					.prepare("SELECT post_id, saved_at FROM post_state WHERE user_key = ?1 AND post_id = ?2")
					.map_err(|e| e.to_string())?;
				for id in ids {
					if let Some((pid, saved_at)) = stmt
						.query_row(params![user_key, id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)))
						.optional()
						.map_err(|e| e.to_string())?
					{
						out.insert(pid, saved_at.is_some());
					}
				}
			}
			tx.commit().map_err(|e| e.to_string())?;
			Ok(out)
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn upsert_seen(&self, user_key: &str, post_ids: &[String], now: i64) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let ids = post_ids.to_vec();
		spawn_blocking(move || {
			let mut conn = pool.get().map_err(|e| e.to_string())?;
			let tx = conn.transaction().map_err(|e| e.to_string())?;
			tx.execute(
				"INSERT OR IGNORE INTO user_session(user_key, created_at, last_seen_at) VALUES(?1, ?2, ?2)",
				params![user_key, now],
			)
			.map_err(|e| e.to_string())?;
			{
				let mut stmt = tx
					.prepare(
						"INSERT INTO post_state(user_key, post_id, first_seen_at, last_seen_at, is_read, saved_at)
                         VALUES(?1, ?2, ?3, ?3, 0, NULL)
                         ON CONFLICT(user_key, post_id) DO UPDATE SET last_seen_at = excluded.last_seen_at",
					)
					.map_err(|e| e.to_string())?;
				for id in ids {
					stmt.execute(params![user_key, id, now]).map_err(|e| e.to_string())?;
				}
			}
			tx.commit().map_err(|e| e.to_string())?;
			Ok(())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn set_archived(&self, user_key: &str, post_id: &str, archived: bool) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let post_id = post_id.to_string();
		let val = if archived { 1i64 } else { 0i64 };
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn.execute(
				"INSERT INTO post_state(user_key, post_id, first_seen_at, last_seen_at, is_read, saved_at, is_archived)
                 VALUES(?1, ?2, strftime('%s','now'), strftime('%s','now'), 0, NULL, ?3)
                 ON CONFLICT(user_key, post_id) DO UPDATE SET is_archived = excluded.is_archived, last_seen_at = excluded.last_seen_at",
				params![user_key, post_id, val],
			)
			.map_err(|e| e.to_string())?;
			Ok(())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn get_archived_set(&self, user_key: &str, post_ids: &[String]) -> Result<std::collections::HashSet<String>, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let ids = post_ids.to_vec();
		spawn_blocking(move || {
			let mut conn = pool.get().map_err(|e| e.to_string())?;
			let mut out = std::collections::HashSet::new();
			let tx = conn.transaction().map_err(|e| e.to_string())?;
			{
				let mut stmt = tx
					.prepare("SELECT post_id FROM post_state WHERE user_key = ?1 AND post_id = ?2 AND is_archived = 1")
					.map_err(|e| e.to_string())?;
				for id in ids {
					if let Some(pid) = stmt
						.query_row(params![user_key, id], |row| row.get::<_, String>(0))
						.optional()
						.map_err(|e| e.to_string())?
					{
						out.insert(pid);
					}
				}
			}
			tx.commit().map_err(|e| e.to_string())?;
			Ok(out)
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn count_unread(&self, user_key: &str) -> Result<i64, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn.query_row(
				"SELECT COUNT(*) FROM post_state WHERE user_key = ?1 AND is_read = 0",
				params![user_key],
				|r| r.get::<_, i64>(0),
			)
			.map_err(|e| e.to_string())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn mark_all_read(&self, user_key: &str) -> Result<usize, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			let n = conn
				.execute("UPDATE post_state SET is_read = 1 WHERE user_key = ?1 AND is_read = 0", params![user_key])
				.map_err(|e| e.to_string())?;
			Ok(n)
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn set_read(&self, user_key: &str, post_id: &str, is_read: bool) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let post_id = post_id.to_string();
		let is_read = if is_read { 1i64 } else { 0i64 };
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn.execute(
				"INSERT INTO post_state(user_key, post_id, first_seen_at, last_seen_at, is_read, saved_at)
                 VALUES(?1, ?2, strftime('%s','now'), strftime('%s','now'), ?3, NULL)
                 ON CONFLICT(user_key, post_id) DO UPDATE SET is_read = excluded.is_read, last_seen_at = excluded.last_seen_at",
				params![user_key, post_id, is_read],
			)
			.map_err(|e| e.to_string())?;
			Ok(())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn set_saved(&self, user_key: &str, post_id: &str, saved: bool) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let post_id = post_id.to_string();
		let is_save = saved;
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			if is_save {
				conn.execute(
					"INSERT INTO post_state(user_key, post_id, first_seen_at, last_seen_at, is_read, saved_at)
                     VALUES(?1, ?2, strftime('%s','now'), strftime('%s','now'), 0, strftime('%s','now'))
                     ON CONFLICT(user_key, post_id) DO UPDATE SET saved_at = excluded.saved_at, last_seen_at = excluded.last_seen_at",
					params![user_key, post_id],
				)
				.map_err(|e| e.to_string())?;
			} else {
				conn.execute(
					"UPDATE post_state SET saved_at = NULL WHERE user_key = ?1 AND post_id = ?2",
					params![user_key, post_id],
				)
				.map_err(|e| e.to_string())?;
			}
			Ok(())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	// ---- Post cache (for saved view) ----

	pub async fn cache_posts(&self, entries: Vec<PostCacheEntry>) -> Result<(), String> {
		if entries.is_empty() {
			return Ok(());
		}
		let pool = self.pool.clone();
		let now = now_ts();
		spawn_blocking(move || {
			let mut conn = pool.get().map_err(|e| e.to_string())?;
			let tx = conn.transaction().map_err(|e| e.to_string())?;
			{
				let mut stmt = tx
					.prepare(
						"INSERT INTO post_cache(post_id, title, community, domain, permalink, score, comments, created_utc, cached_at)
                         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                         ON CONFLICT(post_id) DO UPDATE SET
                             score = excluded.score,
                             comments = excluded.comments,
                             cached_at = excluded.cached_at",
					)
					.map_err(|e| e.to_string())?;
				for e in &entries {
					stmt.execute(params![e.post_id, e.title, e.community, e.domain, e.permalink, e.score, e.comments, e.created_utc, now])
						.map_err(|e| e.to_string())?;
				}
			}
			tx.commit().map_err(|e| e.to_string())?;
			Ok(())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn get_saved_posts(&self, user_key: &str) -> Result<Vec<SavedPostRow>, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			let mut stmt = conn
				.prepare(
					"SELECT ps.post_id, pc.title, pc.community, pc.domain, pc.permalink, pc.score, pc.comments, pc.created_utc, ps.saved_at
                     FROM post_state ps
                     JOIN post_cache pc ON ps.post_id = pc.post_id
                     WHERE ps.user_key = ?1 AND ps.saved_at IS NOT NULL
                     ORDER BY ps.saved_at DESC",
				)
				.map_err(|e| e.to_string())?;
			let rows = stmt
				.query_map(params![user_key], |row| {
					Ok(SavedPostRow {
						post_id: row.get(0)?,
						title: row.get(1)?,
						community: row.get(2)?,
						domain: row.get(3)?,
						permalink: row.get(4)?,
						score: row.get(5)?,
						comments: row.get(6)?,
						created_utc: row.get(7)?,
						saved_at: row.get(8)?,
					})
				})
				.map_err(|e| e.to_string())?;
			let mut out = Vec::new();
			for r in rows {
				out.push(r.map_err(|e| e.to_string())?);
			}
			Ok(out)
		})
		.await
		.map_err(|e| e.to_string())?
	}

	// ---- Channel visits (digest since last visit) ----

	pub async fn get_last_visit(&self, user_key: &str, channel_slug: &str) -> Result<Option<i64>, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let channel_slug = channel_slug.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn
				.query_row(
					"SELECT last_visited_at FROM channel_visit WHERE user_key = ?1 AND channel_slug = ?2",
					params![user_key, channel_slug],
					|row| row.get::<_, i64>(0),
				)
				.optional()
				.map_err(|e| e.to_string())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn update_last_visit(&self, user_key: &str, channel_slug: &str, now: i64) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let channel_slug = channel_slug.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn.execute(
				"INSERT INTO channel_visit(user_key, channel_slug, last_visited_at)
                 VALUES(?1, ?2, ?3)
                 ON CONFLICT(user_key, channel_slug) DO UPDATE SET last_visited_at = excluded.last_visited_at",
				params![user_key, channel_slug, now],
			)
			.map_err(|e| e.to_string())?;
			Ok(())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	// ---- Channels ----

	pub async fn list_channels(&self, user_key: &str) -> Result<Vec<ChannelRow>, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			let mut stmt = conn
				.prepare(
					"SELECT slug, title, rule_json, created_at, updated_at, sort_order FROM channel WHERE user_key = ?1 ORDER BY sort_order ASC, title ASC",
				)
				.map_err(|e| e.to_string())?;
			let rows = stmt
				.query_map(params![user_key], |row| {
					Ok(ChannelRow {
						slug: row.get(0)?,
						title: row.get(1)?,
						rule_json: row.get(2)?,
						created_at: row.get(3)?,
						updated_at: row.get(4)?,
						sort_order: row.get(5)?,
					})
				})
				.map_err(|e| e.to_string())?;
			let mut out = Vec::new();
			for r in rows {
				out.push(r.map_err(|e| e.to_string())?);
			}
			Ok(out)
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn get_channel(&self, user_key: &str, slug: &str) -> Result<Option<ChannelRow>, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let slug = slug.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn
				.query_row(
					"SELECT slug, title, rule_json, created_at, updated_at, sort_order FROM channel WHERE user_key = ?1 AND slug = ?2",
					params![user_key, slug],
					|row| {
						Ok(ChannelRow {
							slug: row.get(0)?,
							title: row.get(1)?,
							rule_json: row.get(2)?,
							created_at: row.get(3)?,
							updated_at: row.get(4)?,
							sort_order: row.get(5)?,
						})
					},
				)
				.optional()
				.map_err(|e| e.to_string())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn upsert_channel(&self, user_key: &str, slug: &str, title: &str, rule_json: &str) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let slug = slug.to_string();
		let title = title.to_string();
		let rule_json = rule_json.to_string();
		let now = now_ts();
		spawn_blocking(move || {
			let mut conn = pool.get().map_err(|e| e.to_string())?;
			let tx = conn.transaction().map_err(|e| e.to_string())?;
			tx.execute(
				"INSERT OR IGNORE INTO user_session(user_key, created_at, last_seen_at) VALUES(?1, ?2, ?2)",
				params![user_key, now],
			)
			.map_err(|e| e.to_string())?;
			tx.execute(
				"INSERT INTO channel(user_key, slug, title, rule_json, created_at, updated_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?5)
                 ON CONFLICT(user_key, slug) DO UPDATE SET title = excluded.title, rule_json = excluded.rule_json, updated_at = excluded.updated_at",
				params![user_key, slug, title, rule_json, now],
			)
			.map_err(|e| e.to_string())?;
			tx.commit().map_err(|e| e.to_string())?;
			Ok(())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	/// Move channel up (-1) or down (+1) in sort_order by swapping with adjacent channel.
	pub async fn move_channel(&self, user_key: &str, slug: &str, direction: i64) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let slug = slug.to_string();
		spawn_blocking(move || {
			let mut conn = pool.get().map_err(|e| e.to_string())?;
			let tx = conn.transaction().map_err(|e| e.to_string())?;
			// Get all channels in order
			let mut channels: Vec<(String, i64)> = {
				let mut stmt = tx.prepare(
					"SELECT slug, sort_order FROM channel WHERE user_key = ?1 ORDER BY sort_order ASC, title ASC"
				).map_err(|e| e.to_string())?;
				let rows = stmt.query_map(params![user_key], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
					.map_err(|e| e.to_string())?;
				let mut out = Vec::new();
				for r in rows { out.push(r.map_err(|e| e.to_string())?); }
				out
			};
			// Find current index
			let idx = match channels.iter().position(|(s, _)| s == &slug) {
				Some(i) => i,
				None => return tx.commit().map_err(|e| e.to_string()),
			};
			let target = if direction < 0 {
				if idx == 0 { return tx.commit().map_err(|e| e.to_string()); }
				idx - 1
			} else {
				if idx + 1 >= channels.len() { return tx.commit().map_err(|e| e.to_string()); }
				idx + 1
			};
			channels.swap(idx, target);
			// Re-assign sort_order 0, 1, 2, ...
			for (i, (s, _)) in channels.iter().enumerate() {
				tx.execute(
					"UPDATE channel SET sort_order = ?1 WHERE user_key = ?2 AND slug = ?3",
					params![i as i64, user_key, s],
				).map_err(|e| e.to_string())?;
			}
			tx.commit().map_err(|e| e.to_string())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	pub async fn delete_channel(&self, user_key: &str, slug: &str) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let slug = slug.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn.execute("DELETE FROM channel WHERE user_key = ?1 AND slug = ?2", params![user_key, slug])
				.map_err(|e| e.to_string())?;
			Ok(())
		})
		.await
		.map_err(|e| e.to_string())?
	}

	// ---- Stats ----

	pub async fn get_stats(&self, user_key: &str) -> Result<ReadingStats, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			let total_seen: i64 = conn
				.query_row("SELECT COUNT(*) FROM post_state WHERE user_key = ?1", params![user_key], |r| r.get(0))
				.unwrap_or(0);
			let total_read: i64 = conn
				.query_row("SELECT COUNT(*) FROM post_state WHERE user_key = ?1 AND is_read = 1", params![user_key], |r| r.get(0))
				.unwrap_or(0);
			let total_saved: i64 = conn
				.query_row("SELECT COUNT(*) FROM post_state WHERE user_key = ?1 AND saved_at IS NOT NULL", params![user_key], |r| r.get(0))
				.unwrap_or(0);
			let total_mutes: i64 = conn
				.query_row("SELECT COUNT(*) FROM mute_rule WHERE user_key = ?1", params![user_key], |r| r.get(0))
				.unwrap_or(0);
			let total_channels: i64 = conn
				.query_row("SELECT COUNT(*) FROM channel WHERE user_key = ?1", params![user_key], |r| r.get(0))
				.unwrap_or(0);
			// Top 10 communities by posts seen
			let mut stmt = conn
				.prepare(
					"SELECT pc.community, COUNT(*) as cnt
					FROM post_state ps JOIN post_cache pc ON ps.post_id = pc.post_id
					WHERE ps.user_key = ?1
					GROUP BY pc.community ORDER BY cnt DESC LIMIT 10",
				)
				.map_err(|e| e.to_string())?;
			let top_communities: Vec<(String, i64)> = stmt
				.query_map(params![user_key], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
				.map_err(|e| e.to_string())?
				.filter_map(|r| r.ok())
				.collect();
			Ok(ReadingStats { total_seen, total_read, total_saved, total_mutes, total_channels, top_communities })
		})
		.await
		.map_err(|e| e.to_string())?
	}

	// ---- Mute export/import ----

	pub async fn export_mutes(&self, user_key: &str) -> Result<Vec<(String, String)>, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			let mut stmt = conn
				.prepare("SELECT rule_type, pattern FROM mute_rule WHERE user_key = ?1 ORDER BY created_at ASC")
				.map_err(|e| e.to_string())?;
			let rows = stmt
				.query_map(params![user_key], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
				.map_err(|e| e.to_string())?
				.filter_map(|r| r.ok())
				.collect();
			Ok(rows)
		})
		.await
		.map_err(|e| e.to_string())?
	}
}

fn migrate(conn: &rusqlite::Connection) -> Result<(), String> {
	let user_version: i64 = conn
		.query_row("PRAGMA user_version", [], |row| row.get(0))
		.map_err(|e| e.to_string())?;

	if user_version >= 5 {
		return Ok(());
	}

	// V1: base tables (user_session, post_state, mute_rule)
	if user_version < 1 {
		conn
			.execute_batch(
				r#"
            BEGIN;
            CREATE TABLE IF NOT EXISTS user_session (
                user_key TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS post_state (
                user_key TEXT NOT NULL,
                post_id TEXT NOT NULL,
                first_seen_at INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL,
                is_read INTEGER NOT NULL DEFAULT 0,
                saved_at INTEGER,
                PRIMARY KEY(user_key, post_id),
                FOREIGN KEY(user_key) REFERENCES user_session(user_key) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS mute_rule (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_key TEXT NOT NULL,
                scope TEXT NOT NULL,
                rule_type TEXT NOT NULL,
                pattern TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(user_key) REFERENCES user_session(user_key) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_post_state_read ON post_state(user_key, is_read, last_seen_at);
            CREATE INDEX IF NOT EXISTS idx_mute_rule_scope ON mute_rule(user_key, scope, rule_type);
            COMMIT;
            "#,
			)
			.map_err(|e| e.to_string())?;
	}

	// V2: channels table
	if user_version < 2 {
		conn
			.execute_batch(
				r#"
            BEGIN;
            CREATE TABLE IF NOT EXISTS channel (
                user_key TEXT NOT NULL,
                slug TEXT NOT NULL,
                title TEXT NOT NULL,
                rule_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY(user_key, slug),
                FOREIGN KEY(user_key) REFERENCES user_session(user_key) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_channel_user ON channel(user_key);
            COMMIT;
            "#,
			)
			.map_err(|e| e.to_string())?;
	}

	// V3: post_cache, channel_visit, unique mute index
	conn
		.execute_batch(
			r#"
        BEGIN;
        CREATE TABLE IF NOT EXISTS post_cache (
            post_id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            community TEXT NOT NULL,
            domain TEXT NOT NULL,
            permalink TEXT NOT NULL,
            score INTEGER NOT NULL DEFAULT 0,
            comments INTEGER NOT NULL DEFAULT 0,
            created_utc INTEGER NOT NULL DEFAULT 0,
            cached_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS channel_visit (
            user_key TEXT NOT NULL,
            channel_slug TEXT NOT NULL,
            last_visited_at INTEGER NOT NULL,
            PRIMARY KEY(user_key, channel_slug),
            FOREIGN KEY(user_key) REFERENCES user_session(user_key) ON DELETE CASCADE
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_mute_unique ON mute_rule(user_key, scope, rule_type, pattern);
        PRAGMA user_version = 3;
        COMMIT;
        "#,
		)
		.map_err(|e| e.to_string())?;

	// V4: sort_order column for channel reordering
	if user_version < 4 {
		conn.execute_batch(
			r#"
			BEGIN;
			ALTER TABLE channel ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
			PRAGMA user_version = 4;
			COMMIT;
			"#,
		)
		.map_err(|e| e.to_string())?;
	}

	// V5: is_archived column for post_state (hide-from-feed without marking read)
	if user_version < 5 {
		conn.execute_batch(
			r#"
			BEGIN;
			ALTER TABLE post_state ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;
			PRAGMA user_version = 5;
			COMMIT;
			"#,
		)
		.map_err(|e| e.to_string())?;
	}

	Ok(())
}

fn prune_old_state(conn: &rusqlite::Connection) -> Result<(), String> {
	// Keep 90 days of post_state; never delete saved posts
	let cutoff = now_ts() - (90 * 24 * 3600);
	conn.execute(
		"DELETE FROM post_state WHERE last_seen_at < ?1 AND saved_at IS NULL",
		params![cutoff],
	)
	.map_err(|e| e.to_string())?;
	// Prune post_cache entries older than 90 days with no associated post_state
	conn.execute(
		"DELETE FROM post_cache WHERE cached_at < ?1 AND post_id NOT IN (SELECT post_id FROM post_state)",
		params![cutoff],
	)
	.map_err(|e| e.to_string())?;
	Ok(())
}
