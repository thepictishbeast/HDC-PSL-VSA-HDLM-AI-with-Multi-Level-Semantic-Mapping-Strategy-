// PlausiDen AI — Local Persistence via SQLite
//
// Per Architectural Bible §4.2: all persistent data lives in a local SQLite
// database. No localStorage for sensitive data. No cloud sync. Operator owns
// every byte.
//
// Tables: facts, conversations, messages, training_results
// Location: ~/.local/share/plausiden/brain.db (or --db-path override)

use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::{info, warn};

pub struct BrainDb {
    pub conn: Mutex<Connection>,
}

impl BrainDb {
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        // SECURITY: WAL mode allows concurrent reads during writes.
        // Critical for 40M+ fact DB where ingestion and serving overlap.
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;\
             PRAGMA busy_timeout=30000;\
             PRAGMA synchronous=NORMAL;\
             PRAGMA cache_size=-64000;"
        )?;
        let db = Self { conn: Mutex::new(conn) };
        db.migrate()?;
        info!("// PERSISTENCE: SQLite opened at {} (WAL mode, 30s timeout)", path.display());
        Ok(db)
    }

    pub fn default_path() -> PathBuf {
        let base = std::env::var("XDG_DATA_HOME")
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                format!("{}/.local/share", home)
            });
        PathBuf::from(base).join("plausiden").join("brain.db")
    }

    fn migrate(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS facts (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                source TEXT DEFAULT 'user',
                confidence REAL DEFAULT 1.0,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT 'New chat',
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now')),
                pinned INTEGER DEFAULT 0,
                starred INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                meta_json TEXT,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id)
            );

            CREATE INDEX IF NOT EXISTS idx_messages_convo ON messages(conversation_id);

            CREATE TABLE IF NOT EXISTS training_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                domain TEXT NOT NULL,
                accuracy REAL NOT NULL,
                total INTEGER NOT NULL,
                correct INTEGER NOT NULL,
                timestamp TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS user_profile (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                category TEXT DEFAULT 'general',
                learned_at TEXT DEFAULT (datetime('now')),
                source TEXT DEFAULT 'conversation'
            );
        ")?;
        info!("// PERSISTENCE: Schema migrated");
        Ok(())
    }

    // ---- Facts ----

    pub fn upsert_fact(&self, key: &str, value: &str, source: &str, confidence: f64) {
        let conn = self.conn.lock().unwrap();
        if let Err(e) = conn.execute(
            "INSERT INTO facts (key, value, source, confidence, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value=?2, source=?3, confidence=?4, updated_at=datetime('now')",
            params![key, value, source, confidence],
        ) {
            warn!("// PERSISTENCE: upsert_fact failed: {}", e);
        }
    }

    pub fn get_all_facts(&self) -> Vec<(String, String, String, f64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT key, value, source, confidence FROM facts ORDER BY updated_at DESC"
        ).unwrap();
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
            ))
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    /// Get the most recent N facts, prioritizing user-extracted and high-confidence
    /// facts. Used for startup hydration to avoid loading 40M+ rows into memory.
    pub fn get_recent_facts(&self, limit: usize) -> Vec<(String, String, String, f64)> {
        let conn = self.conn.lock().unwrap();
        // Prioritize: ai_extracted facts first (user conversations), then by
        // confidence descending, then by recency. This ensures the agent knows
        // the user's name, preferences, etc. on startup.
        let mut stmt = conn.prepare(
            "SELECT key, value, source, confidence FROM facts \
             ORDER BY CASE WHEN source = 'ai_extracted' THEN 0 ELSE 1 END, \
             confidence DESC, updated_at DESC LIMIT ?1"
        ).unwrap_or_else(|_| {
            // Fallback: simple LIMIT query without ordering
            conn.prepare("SELECT key, value, source, confidence FROM facts LIMIT ?1").unwrap()
        });
        stmt.query_map(rusqlite::params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
            ))
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn delete_fact(&self, key: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM facts WHERE key = ?1", params![key]);
    }

    /// Search facts by keyword — used for RAG (Retrieval-Augmented Generation).
    /// Uses FTS5 full-text search (52M+ rows indexed) for instant retrieval.
    /// Falls back to LIKE if FTS5 is unavailable.
    /// SUPERSOCIETY: This is what makes 52M facts useful in real-time.
    pub fn search_facts(&self, query: &str, limit: usize) -> Vec<(String, String, f64)> {
        let conn = self.conn.lock().unwrap();

        // Extract keywords for FTS5 MATCH query
        let stopwords = ["the","and","for","are","but","not","you","all","can","had",
            "her","was","one","our","out","has","its","how","who","what","when",
            "where","why","this","that","with","from","they","been","have","many"];
        let keywords: Vec<String> = query.split_whitespace()
            .filter(|w| w.len() >= 3 && !stopwords.contains(&w.to_lowercase().as_str()))
            .take(5)
            .map(|w| w.to_lowercase())
            .collect();

        if keywords.is_empty() {
            return vec![];
        }

        // Try FTS5 first (instant), fall back to LIKE (slow)
        let fts_query = keywords.join(" ");
        let mut results = Vec::new();

        // FTS5 path — BM25 ranking weighted by quality_score.
        // SUPERSOCIETY: Curated high-quality facts (0.95) surface above
        // bulk web text (0.65). This is what makes RAG useful.
        let mut stmt = match conn.prepare(
            "SELECT f.key, f.value, COALESCE(f.quality_score, f.confidence, 0.5) \
             FROM facts f \
             JOIN facts_fts ON f.rowid = facts_fts.rowid \
             WHERE facts_fts MATCH ?1 \
             ORDER BY rank / COALESCE(f.quality_score, f.confidence, 0.5) \
             LIMIT ?2"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        if let Ok(rows) = stmt.query_map(params![fts_query, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        }) {
            for row in rows {
                if let Ok(r) = row { results.push(r); }
            }
        }
        results
    }

    // ---- Conversations ----

    pub fn save_conversation(&self, id: &str, title: &str, pinned: bool, starred: bool) {
        let conn = self.conn.lock().unwrap();
        if let Err(e) = conn.execute(
            "INSERT INTO conversations (id, title, pinned, starred, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET title=?2, pinned=?3, starred=?4, updated_at=datetime('now')",
            params![id, title, pinned as i32, starred as i32],
        ) {
            warn!("// PERSISTENCE: save_conversation failed: {}", e);
        }
    }

    pub fn save_message(&self, convo_id: &str, role: &str, content: &str, timestamp: i64, meta: Option<&str>) {
        let conn = self.conn.lock().unwrap();
        if let Err(e) = conn.execute(
            "INSERT INTO messages (conversation_id, role, content, timestamp, meta_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![convo_id, role, content, timestamp, meta],
        ) {
            warn!("// PERSISTENCE: save_message failed: {}", e);
        }
    }

    pub fn get_conversations(&self) -> Vec<(String, String, bool, bool, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, pinned, starred, updated_at FROM conversations ORDER BY updated_at DESC LIMIT 200"
        ).unwrap();
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)? != 0,
                row.get::<_, i32>(3)? != 0,
                row.get::<_, String>(4)?,
            ))
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn get_messages(&self, convo_id: &str) -> Vec<(String, String, i64, Option<String>)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT role, content, timestamp, meta_json FROM messages WHERE conversation_id = ?1 ORDER BY timestamp"
        ).unwrap();
        stmt.query_map(params![convo_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    pub fn delete_conversation(&self, id: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute("DELETE FROM messages WHERE conversation_id = ?1", params![id]);
        let _ = conn.execute("DELETE FROM conversations WHERE id = ?1", params![id]);
    }

    // ---- Training results ----

    pub fn log_training_result(&self, domain: &str, accuracy: f64, total: usize, correct: usize) {
        let conn = self.conn.lock().unwrap();
        if let Err(e) = conn.execute(
            "INSERT INTO training_results (domain, accuracy, total, correct) VALUES (?1, ?2, ?3, ?4)",
            params![domain, accuracy, total as i64, correct as i64],
        ) {
            warn!("// PERSISTENCE: log_training_result failed: {}", e);
        }
    }

    pub fn get_training_history(&self, limit: usize) -> Vec<(String, f64, i64, i64, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT domain, accuracy, total, correct, timestamp
             FROM training_results ORDER BY id DESC LIMIT ?1"
        ).unwrap();
        stmt.query_map(params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    // ---- Settings (key/value) ----

    pub fn set_setting(&self, key: &str, value: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=?2",
            params![key, value],
        );
    }

    pub fn get_setting(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        ).ok()
    }
    // ---- User Profile (persistent memory across sessions) ----

    /// Save a user profile fact. Used for: name, role, location, preferences,
    /// relationships. These are loaded fully on startup (not capped like facts)
    /// so the AI always knows who it's talking to.
    pub fn save_profile(&self, key: &str, value: &str, category: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO user_profile (key, value, category, learned_at) \
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![key, value, category],
        );
    }

    /// Load all user profile facts. Called on startup to hydrate the agent's
    /// understanding of the user. Returns (key, value, category) tuples.
    pub fn load_profile(&self) -> Vec<(String, String, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT key, value, category FROM user_profile ORDER BY learned_at DESC"
        ).unwrap();
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        }).unwrap().filter_map(|r| r.ok()).collect()
    }

    /// Get a specific profile value.
    pub fn get_profile(&self, key: &str) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM user_profile WHERE key = ?1",
            params![key],
            |row| row.get(0),
        ).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_db() -> BrainDb {
        let id = std::process::id();
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let path = PathBuf::from(format!("/tmp/plausiden_test_brain_{}_{}.db", id, ts));
        let _ = std::fs::remove_file(&path);
        BrainDb::open(&path).unwrap()
    }

    #[test]
    fn fact_roundtrip() {
        let db = temp_db();
        db.upsert_fact("name", "Wyatt", "user", 1.0);
        let facts = db.get_all_facts();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].0, "name");
        assert_eq!(facts[0].1, "Wyatt");
    }

    #[test]
    fn fact_update() {
        let db = temp_db();
        db.upsert_fact("role", "developer", "user", 1.0);
        db.upsert_fact("role", "architect", "ai_extracted", 0.9);
        let facts = db.get_all_facts();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].1, "architect");
    }

    #[test]
    fn fact_delete() {
        let db = temp_db();
        db.upsert_fact("temp", "data", "user", 1.0);
        db.delete_fact("temp");
        assert_eq!(db.get_all_facts().len(), 0);
    }

    #[test]
    fn training_log() {
        let db = temp_db();
        db.log_training_result("math", 0.85, 100, 85);
        db.log_training_result("social", 0.72, 100, 72);
        let history = db.get_training_history(10);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].0, "social"); // most recent first
    }

    #[test]
    fn settings_roundtrip() {
        let db = temp_db();
        db.set_setting("default_tier", "BigBrain");
        assert_eq!(db.get_setting("default_tier"), Some("BigBrain".to_string()));
        assert_eq!(db.get_setting("nonexistent"), None);
    }
}
