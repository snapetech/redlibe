use crate::config::get_setting;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;
use tokio::task::spawn_blocking;

#[derive(Debug, Clone)]
pub struct MuteRule {
	pub rule_type: String,
	pub pattern: String,
}

#[derive(Clone)]
pub struct SqliteStore {
	pool: Pool<SqliteConnectionManager>,
}

impl SqliteStore {
	pub fn from_env() -> Result<Self, String> {
		let db_path = get_setting("REDLIB_DB_PATH").unwrap_or_else(|| "redlib.sqlite".into());
		let mgr = SqliteConnectionManager::file(db_path);
		let pool = Pool::builder()
			.max_size(16)
			.build(mgr)
			.map_err(|e| e.to_string())?;

		// Run migrations eagerly
		{
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn
				.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")
				.map_err(|e| e.to_string())?;
			migrate(&conn)?;
		}

		Ok(Self { pool })
	}

	pub async fn list_mutes(&self, user_key: &str, scope: &str) -> Result<Vec<MuteRule>, String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let scope = scope.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			let mut stmt = conn
				.prepare("SELECT rule_type, pattern FROM mute_rule WHERE user_key = ?1 AND scope = ?2")
				.map_err(|e| e.to_string())?;
			let rows = stmt
				.query_map(params![user_key, scope], |row| {
					Ok(MuteRule {
						rule_type: row.get(0)?,
						pattern: row.get(1)?,
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

	pub async fn add_mute(&self, user_key: &str, scope: &str, rule_type: &str, pattern: &str) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let scope = scope.to_string();
		let rule_type = rule_type.to_string();
		let pattern = pattern.to_string();
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			conn.execute(
				"INSERT INTO mute_rule(user_key, scope, rule_type, pattern, created_at) VALUES(?1, ?2, ?3, ?4, strftime('%s','now'))",
				params![user_key, scope, rule_type, pattern],
			)
			.map_err(|e| e.to_string())?;
			Ok(())
		})
		.await
		.map_err(|e| e.to_string())?
	}

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

	pub async fn upsert_seen(&self, user_key: &str, post_ids: &[String], now_ts: i64) -> Result<(), String> {
		let pool = self.pool.clone();
		let user_key = user_key.to_string();
		let ids = post_ids.to_vec();
		spawn_blocking(move || {
			let mut conn = pool.get().map_err(|e| e.to_string())?;
			let tx = conn.transaction().map_err(|e| e.to_string())?;
			tx.execute(
				"INSERT OR IGNORE INTO user_session(user_key, created_at, last_seen_at) VALUES(?1, ?2, ?2)",
				params![user_key, now_ts],
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
					stmt.execute(params![user_key, id, now_ts]).map_err(|e| e.to_string())?;
				}
			}
			tx.commit().map_err(|e| e.to_string())?;
			Ok(())
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
		let saved_at: Option<i64> = if saved { Some(0) } else { None };
		spawn_blocking(move || {
			let conn = pool.get().map_err(|e| e.to_string())?;
			if saved_at.is_some() {
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
}

fn migrate(conn: &rusqlite::Connection) -> Result<(), String> {
	let user_version: i64 = conn
		.query_row("PRAGMA user_version", [], |row| row.get(0))
		.map_err(|e| e.to_string())?;

	if user_version >= 1 {
		return Ok(());
	}

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

        PRAGMA user_version = 1;
        COMMIT;
        "#,
		)
		.map_err(|e| e.to_string())?;

	Ok(())
}
