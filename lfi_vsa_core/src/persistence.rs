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
        // SECURITY: WAL mode — concurrent reads during writes; required for
        // 75GB+ fact DB where ingestion and serving overlap.
        //
        // PERFORMANCE: tuning chosen for read-heavy 75GB workload on a box
        // with 16GB+ RAM. See docs (or PRAGMA comments below) for rationale.
        // AVP-PASS-1: each PRAGMA is documented so future tuners know what
        // to keep and what is negotiable (#319).
        //
        //   journal_mode=WAL         concurrent reads during writes
        //   busy_timeout=30000       survive WAL checkpoints without EBUSY
        //   synchronous=NORMAL       WAL-safe durability; ~2x writes vs FULL
        //   cache_size=-262144       256 MB page cache (hot fact working set)
        //   mmap_size=8589934592     8 GB mmap — zero-copy reads for FTS5
        //                            (kernel handles page cache; freed on
        //                            memory pressure, no OOM risk)
        //   temp_store=MEMORY        FTS5/ORDER BY scratch in RAM, not disk
        //   page_size=8192           only effective on a pristine DB; current
        //                            prod DB is locked to whatever it was
        //                            created with — changing requires VACUUM
        //   wal_autocheckpoint=0     block startup checkpoints; nightly cron
        //                            owns WAL checkpointing for the big DB
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;\
             PRAGMA busy_timeout=30000;\
             PRAGMA synchronous=NORMAL;\
             PRAGMA cache_size=-262144;\
             PRAGMA mmap_size=8589934592;\
             PRAGMA temp_store=MEMORY;\
             PRAGMA page_size=8192;\
             PRAGMA wal_autocheckpoint=0;"
        )?;
        let db = Self { conn: Mutex::new(conn) };
        db.migrate()?;
        info!("// PERSISTENCE: SQLite opened at {} (WAL, 30s busy_timeout, 256MB cache, 8GB mmap)", path.display());
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
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
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

            CREATE TABLE IF NOT EXISTS fact_edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source_key TEXT NOT NULL,
                target_key TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                strength REAL DEFAULT 0.5,
                evidence TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                UNIQUE(source_key, target_key, edge_type)
            );

            CREATE INDEX IF NOT EXISTS idx_edges_source ON fact_edges(source_key);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON fact_edges(target_key);
            CREATE INDEX IF NOT EXISTS idx_edges_type ON fact_edges(edge_type);

            CREATE TABLE IF NOT EXISTS domain_xref (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                domain_a TEXT NOT NULL,
                domain_b TEXT NOT NULL,
                shared_concept TEXT NOT NULL,
                strength REAL DEFAULT 0.5,
                example_fact_key TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                UNIQUE(domain_a, domain_b, shared_concept)
            );

            CREATE INDEX IF NOT EXISTS idx_xref_domains ON domain_xref(domain_a, domain_b);

            CREATE TABLE IF NOT EXISTS fact_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                fact_key TEXT NOT NULL,
                old_value TEXT,
                new_value TEXT NOT NULL,
                old_quality REAL,
                new_quality REAL,
                change_type TEXT NOT NULL,
                changed_by TEXT DEFAULT 'system',
                reason TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_versions_key ON fact_versions(fact_key);
            CREATE INDEX IF NOT EXISTS idx_versions_time ON fact_versions(created_at);

            -- #298 Contradiction ledger. Row is inserted at ingest time
            -- when an incoming high-confidence fact for a key disagrees
            -- with an existing high-confidence fact. Unlike fact_versions
            -- (which records all changes), this is limited to the subset
            -- that warrants human / axiom review.
            CREATE TABLE IF NOT EXISTS contradictions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                fact_key TEXT NOT NULL,
                existing_value TEXT NOT NULL,
                incoming_value TEXT NOT NULL,
                existing_confidence REAL NOT NULL,
                incoming_confidence REAL NOT NULL,
                existing_source TEXT,
                incoming_source TEXT,
                resolved_at TEXT,
                resolved_value TEXT,
                detected_at TEXT DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_contradictions_key ON contradictions(fact_key);
            CREATE INDEX IF NOT EXISTS idx_contradictions_detected ON contradictions(detected_at);
            CREATE INDEX IF NOT EXISTS idx_contradictions_unresolved
                ON contradictions(fact_key) WHERE resolved_at IS NULL;

            -- #305 Merkle-chained security audit log. Each row binds to
            -- the previous via SHA-256(index || ts || category || actor ||
            -- action || detail || prev_hash). Any modification to an older
            -- row breaks the chain; verify_chain() reports the first
            -- broken index.
            --
            -- Distinct from `audit_log` (AVP audit-pass records) —
            -- chain entries are security events: auth success/fail, tier
            -- changes, fact deletions, policy flips, contradiction
            -- resolutions, etc. Append-only from the Rust side; SQL
            -- DELETE / UPDATE is detected by verify_chain even if
            -- a malicious operator gets DB write access.
            CREATE TABLE IF NOT EXISTS security_audit_chain (
                idx INTEGER PRIMARY KEY AUTOINCREMENT,
                ts_ms INTEGER NOT NULL,
                category TEXT NOT NULL,
                severity TEXT NOT NULL,
                actor TEXT NOT NULL,
                action TEXT NOT NULL,
                detail TEXT NOT NULL,
                prev_hash BLOB NOT NULL,
                entry_hash BLOB NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sac_ts ON security_audit_chain(ts_ms);
            CREATE INDEX IF NOT EXISTS idx_sac_cat ON security_audit_chain(category);
            CREATE INDEX IF NOT EXISTS idx_sac_actor ON security_audit_chain(actor);

            -- #303 Capability tokens: bearer credentials granting a
            -- named capability (ingest, admin_read, chain_append, etc).
            -- Distinct from the sovereign passphrase: tokens can be
            -- rotated, scoped, and revoked without changing root auth.
            -- Token value is stored as SHA-256 hash so a DB leak does
            -- not hand over live tokens.
            CREATE TABLE IF NOT EXISTS capability_tokens (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                token_hash BLOB NOT NULL UNIQUE,
                capability TEXT NOT NULL,
                label TEXT,
                issued_at TEXT DEFAULT (datetime('now')),
                expires_at TEXT,
                revoked_at TEXT,
                last_used_at TEXT,
                use_count INTEGER DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_capability_tokens_hash
                ON capability_tokens(token_hash);
            CREATE INDEX IF NOT EXISTS idx_capability_tokens_cap
                ON capability_tokens(capability);

            -- #326 Ingestion batch control surface — tracks active +
            -- historical ingest runs so the UI can show progress, start
            -- new batches, and stop in-flight ones. Decoupled from the
            -- Python ingest scripts themselves: scripts register their
            -- run_id + metadata here; the UI reads from the table.
            CREATE TABLE IF NOT EXISTS ingest_batches (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL UNIQUE,
                corpus TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'running',
                tuples_requested INTEGER DEFAULT 0,
                tuples_ingested INTEGER DEFAULT 0,
                psl_pass_rate REAL,
                started_at TEXT DEFAULT (datetime('now')),
                completed_at TEXT,
                exit_reason TEXT,
                pid INTEGER,
                notes TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_ingest_status ON ingest_batches(status);
            CREATE INDEX IF NOT EXISTS idx_ingest_started ON ingest_batches(started_at);
            CREATE INDEX IF NOT EXISTS idx_ingest_corpus ON ingest_batches(corpus);

            -- #293 Per-source trust weights used for cross-source fact
            -- reconciliation. 0.0 = adversarial/untrusted,
            -- 1.0 = fully trusted. Unknown sources default to 0.5 via
            -- the trust_of() helper. Populated on-demand by the
            -- reconciliation pass and by the trust API.
            CREATE TABLE IF NOT EXISTS source_trust (
                source TEXT PRIMARY KEY,
                trust REAL NOT NULL DEFAULT 0.5,
                notes TEXT,
                updated_at TEXT DEFAULT (datetime('now'))
            );

            -- #337 FSRS fact review scheduler persistence.
            -- Keyed by fact_key so every fact can be independently
            -- scheduled. State fields match FsrsCard in
            -- cognition/fsrs_scheduler.rs so round-tripping is lossless.
            CREATE TABLE IF NOT EXISTS fsrs_cards (
                fact_key TEXT PRIMARY KEY,
                difficulty REAL NOT NULL DEFAULT 5.0,
                stability REAL NOT NULL DEFAULT 0.4072,
                last_review INTEGER NOT NULL DEFAULT 0,
                review_count INTEGER NOT NULL DEFAULT 0,
                lapses INTEGER NOT NULL DEFAULT 0,
                state TEXT NOT NULL DEFAULT 'new',
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_fsrs_last_review ON fsrs_cards(last_review);
            CREATE INDEX IF NOT EXISTS idx_fsrs_state ON fsrs_cards(state);

            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                audit_type TEXT NOT NULL,
                pass_number INTEGER NOT NULL,
                tier INTEGER NOT NULL,
                status TEXT NOT NULL,
                findings_total INTEGER DEFAULT 0,
                findings_fixed INTEGER DEFAULT 0,
                findings_open INTEGER DEFAULT 0,
                score REAL,
                details TEXT,
                started_at TEXT DEFAULT (datetime('now')),
                completed_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_audit_type ON audit_log(audit_type);
            CREATE INDEX IF NOT EXISTS idx_audit_tier ON audit_log(tier);

            CREATE TABLE IF NOT EXISTS training_provenance (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                source_file TEXT,
                domain TEXT,
                pairs_used INTEGER DEFAULT 0,
                accuracy_before REAL,
                accuracy_after REAL,
                accuracy_delta REAL,
                model TEXT,
                notes TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_prov_run ON training_provenance(run_id);
            CREATE INDEX IF NOT EXISTS idx_prov_domain ON training_provenance(domain);

            -- User feedback (#350): in-conversation training signal.
            -- rating='up' / 'down' / 'correct'. correction = the response
            -- the user says LFI SHOULD have given. conclusion_id ties the
            -- feedback to a specific reasoning trace.
            CREATE TABLE IF NOT EXISTS user_feedback (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT,
                message_id TEXT,
                conclusion_id INTEGER,
                user_query TEXT,
                lfi_reply TEXT,
                rating TEXT NOT NULL,
                correction TEXT,
                comment TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                processed_at TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_feedback_conv ON user_feedback(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_feedback_rating ON user_feedback(rating);
            CREATE INDEX IF NOT EXISTS idx_feedback_created ON user_feedback(created_at);
            CREATE INDEX IF NOT EXISTS idx_feedback_processed ON user_feedback(processed_at);
        ")?;

        // REGRESSION-GUARD: facts_fts + sync triggers were historically created
        // out-of-band (one-shot sqlite3 command), so fresh installs silently had
        // zero full-text recall. Creating them IF NOT EXISTS is idempotent and
        // covers both pristine and already-populated databases.
        // AVP-PASS-1: surfaced while writing search_facts_expanded tests (#294).
        let fts_setup = conn.execute_batch("
            CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
                value, content=facts, content_rowid=rowid
            );

            CREATE TRIGGER IF NOT EXISTS facts_ai AFTER INSERT ON facts BEGIN
                INSERT INTO facts_fts(rowid, value) VALUES (new.rowid, new.value);
            END;
            CREATE TRIGGER IF NOT EXISTS facts_ad AFTER DELETE ON facts BEGIN
                INSERT INTO facts_fts(facts_fts, rowid, value) VALUES('delete', old.rowid, old.value);
            END;
            CREATE TRIGGER IF NOT EXISTS facts_au AFTER UPDATE ON facts BEGIN
                INSERT INTO facts_fts(facts_fts, rowid, value) VALUES('delete', old.rowid, old.value);
                INSERT INTO facts_fts(rowid, value) VALUES (new.rowid, new.value);
            END;

            -- Fact versioning: auto-record the pre-image whenever a fact's
            -- value or confidence changes. Skips no-op updates (touching
            -- updated_at without content change) so the history isn't
            -- polluted with non-changes.
            -- REGRESSION-GUARD: references only columns present in the base
            -- migrate() schema — prod has extras (quality_score etc.) added
            -- out-of-band; a trigger referencing those would crash inserts
            -- on fresh DBs. See #321 for the broader drift issue.
            -- AVP-PASS-1: auditable history trail for every fact mutation (#292).
            CREATE TRIGGER IF NOT EXISTS facts_version_au AFTER UPDATE ON facts
            WHEN OLD.value IS NOT NEW.value
              OR OLD.confidence IS NOT NEW.confidence
            BEGIN
                INSERT INTO fact_versions(
                    fact_key, old_value, new_value,
                    old_quality, new_quality,
                    change_type, changed_by, reason, created_at
                ) VALUES (
                    OLD.key, OLD.value, NEW.value,
                    OLD.confidence,
                    NEW.confidence,
                    'updated',
                    COALESCE(NEW.source, 'system'),
                    NULL,
                    datetime('now')
                );
            END;
        ");
        match &fts_setup {
            Ok(_) => {
                // Verify the virtual table is actually reachable — catches
                // silent rusqlite/SQLite builds without fts5 compiled in.
                let probe: Result<i64, _> = conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name='facts_fts'",
                    [], |r| r.get(0));
                if probe.unwrap_or(0) == 0 {
                    warn!("// PERSISTENCE: facts_fts create reported OK but not in sqlite_master — RAG disabled");
                }
            }
            Err(e) => {
                // FTS5 feature must be available at the rusqlite/SQLite link level.
                // Warn rather than fail migrate() so the DB still opens for
                // non-RAG reads; search_facts will return an empty vec.
                warn!("// PERSISTENCE: facts_fts setup failed (RAG disabled): {}", e);
            }
        }

        // #295 Idempotent column add for the HDC vector cache. SQLite 3.35+
        // supports O(1) ADD COLUMN when the new column is nullable with no
        // default — which is exactly what we want. This MUST happen once
        // at open time; doing it inside set_fact_vector at every call
        // would re-take the writer lock and hang on a 58M-row table.
        let _ = conn.execute("ALTER TABLE facts ADD COLUMN hdc_vector BLOB", []);

        // #321 quality_score was added to prod via an external ALTER
        // (not in the base CREATE TABLE), so fresh DBs never have it and
        // search_facts's "COALESCE(quality_score, ...)" fails to prepare.
        // Idempotent add here ensures parity. Same O(1) SQLite 3.35+ guarantee.
        let _ = conn.execute("ALTER TABLE facts ADD COLUMN quality_score REAL", []);

        info!("// PERSISTENCE: Schema migrated");
        Ok(())
    }

    // ---- Facts ----

    pub fn upsert_fact(&self, key: &str, value: &str, source: &str, confidence: f64) {
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        // #298 Contradiction detector: before upsert, check whether the
        // existing row for this key holds a *different* high-confidence
        // value. If so, log the disagreement. Bounds:
        //  - both existing AND incoming confidence must be ≥ 0.7
        //  - a trivial re-ingest of the same value does NOT fire
        //  - no-op if the key is new (no existing row)
        //
        // Both reads + writes happen inside the same lock; no TOCTOU.
        let existing: Option<(String, f64, String)> = conn.query_row(
            "SELECT value, confidence, COALESCE(source,'') FROM facts WHERE key = ?1",
            params![key],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, String>(2)?)),
        ).ok();
        if let Some((ev, ec, es)) = existing.as_ref() {
            if ev != value && *ec >= 0.7 && confidence >= 0.7 {
                let _ = conn.execute(
                    "INSERT INTO contradictions \
                     (fact_key, existing_value, incoming_value, \
                      existing_confidence, incoming_confidence, \
                      existing_source, incoming_source) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![key, ev, value, ec, confidence, es, source],
                );
            }
        }

        if let Err(e) = conn.execute(
            "INSERT INTO facts (key, value, source, confidence, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value=?2, source=?3, confidence=?4, updated_at=datetime('now')",
            params![key, value, source, confidence],
        ) {
            warn!("// PERSISTENCE: upsert_fact failed: {}", e);
        }
    }

    /// #298 List recent unresolved contradictions for review UI.
    /// BUG ASSUMPTION: unbounded `limit` could OOM with a pathological DB;
    /// caller MUST cap (API layer does — 500).
    pub fn recent_contradictions(&self, limit: i64, only_unresolved: bool)
        -> Vec<(i64, String, String, String, f64, f64, Option<String>, Option<String>,
                String, Option<String>, Option<String>)>
    {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let sql = if only_unresolved {
            "SELECT id, fact_key, existing_value, incoming_value, \
                    existing_confidence, incoming_confidence, \
                    existing_source, incoming_source, \
                    detected_at, resolved_at, resolved_value \
             FROM contradictions WHERE resolved_at IS NULL \
             ORDER BY detected_at DESC LIMIT ?1"
        } else {
            "SELECT id, fact_key, existing_value, incoming_value, \
                    existing_confidence, incoming_confidence, \
                    existing_source, incoming_source, \
                    detected_at, resolved_at, resolved_value \
             FROM contradictions ORDER BY detected_at DESC LIMIT ?1"
        };
        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s, Err(_) => return Vec::new(),
        };
        stmt.query_map(params![limit], |r| Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, f64>(4)?,
            r.get::<_, f64>(5)?,
            r.get::<_, Option<String>>(6)?,
            r.get::<_, Option<String>>(7)?,
            r.get::<_, String>(8)?,
            r.get::<_, Option<String>>(9)?,
            r.get::<_, Option<String>>(10)?,
        ))).map(|i| i.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    /// #298 Resolve a flagged contradiction by picking the canonical
    /// value. The resolver is the caller's responsibility (user UI,
    /// PSL axiom, metacognitive calibrator). We only persist the verdict.
    pub fn resolve_contradiction(&self, id: i64, resolved_value: &str) -> bool {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE contradictions SET resolved_at = datetime('now'), \
             resolved_value = ?2 WHERE id = ?1",
            params![id, resolved_value],
        ).map(|n| n > 0).unwrap_or(false)
    }

    // ---- Merkle-chained security audit log (#305) ----
    //
    // entry_hash = SHA-256(idx_be || ts_be || cat || sev || actor ||
    //                      action || detail || prev_hash)
    // Fields separated by a 0x1F (unit separator) to eliminate field
    // boundary ambiguity in concatenation. First row's prev_hash is
    // all-zeros.

    /// Append a security audit event. Hash-chains to the previous row.
    /// Returns (idx, entry_hash) on success, None on DB failure.
    pub fn audit_chain_append(
        &self,
        category: &str, severity: &str, actor: &str,
        action: &str, detail: &str,
    ) -> Option<(i64, Vec<u8>)> {
        use sha2::{Sha256, Digest};
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64).unwrap_or(0);

        // Look up previous hash (or zeros for first row).
        let (prev_idx, prev_hash): (i64, Vec<u8>) = conn.query_row(
            "SELECT idx, entry_hash FROM security_audit_chain \
             ORDER BY idx DESC LIMIT 1",
            [],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?)),
        ).unwrap_or((0, vec![0u8; 32]));
        let new_idx = prev_idx + 1;

        let mut hasher = Sha256::new();
        hasher.update(new_idx.to_be_bytes());
        hasher.update(&[0x1Fu8]);
        hasher.update(ts_ms.to_be_bytes());
        hasher.update(&[0x1Fu8]);
        hasher.update(category.as_bytes());
        hasher.update(&[0x1Fu8]);
        hasher.update(severity.as_bytes());
        hasher.update(&[0x1Fu8]);
        hasher.update(actor.as_bytes());
        hasher.update(&[0x1Fu8]);
        hasher.update(action.as_bytes());
        hasher.update(&[0x1Fu8]);
        hasher.update(detail.as_bytes());
        hasher.update(&[0x1Fu8]);
        hasher.update(&prev_hash);
        let entry_hash: Vec<u8> = hasher.finalize().to_vec();

        let rows = conn.execute(
            "INSERT INTO security_audit_chain \
             (ts_ms, category, severity, actor, action, detail, prev_hash, entry_hash) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![ts_ms, category, severity, actor, action, detail,
                    prev_hash, &entry_hash],
        ).ok()?;
        if rows == 0 { return None; }
        Some((conn.last_insert_rowid(), entry_hash))
    }

    /// Walk the chain and verify every link. Returns Ok(entry_count) on
    /// a fully-valid chain or Err(first_bad_idx) when a link breaks.
    pub fn audit_chain_verify(&self) -> Result<i64, i64> {
        use sha2::{Sha256, Digest};
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT idx, ts_ms, category, severity, actor, action, detail, \
                    prev_hash, entry_hash \
             FROM security_audit_chain ORDER BY idx ASC"
        ) { Ok(s) => s, Err(_) => return Ok(0) };
        let rows: Vec<_> = stmt.query_map([], |r| Ok((
            r.get::<_, i64>(0)?, r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?, r.get::<_, String>(3)?,
            r.get::<_, String>(4)?, r.get::<_, String>(5)?,
            r.get::<_, String>(6)?,
            r.get::<_, Vec<u8>>(7)?, r.get::<_, Vec<u8>>(8)?,
        ))).map(|i| i.filter_map(|r| r.ok()).collect()).unwrap_or_default();

        let mut expected_prev: Vec<u8> = vec![0u8; 32];
        for (idx, ts_ms, cat, sev, actor, action, detail, prev, entry) in rows.iter() {
            if prev != &expected_prev { return Err(*idx); }
            let mut hasher = Sha256::new();
            hasher.update(idx.to_be_bytes());
            hasher.update(&[0x1Fu8]);
            hasher.update(ts_ms.to_be_bytes());
            hasher.update(&[0x1Fu8]);
            hasher.update(cat.as_bytes());
            hasher.update(&[0x1Fu8]);
            hasher.update(sev.as_bytes());
            hasher.update(&[0x1Fu8]);
            hasher.update(actor.as_bytes());
            hasher.update(&[0x1Fu8]);
            hasher.update(action.as_bytes());
            hasher.update(&[0x1Fu8]);
            hasher.update(detail.as_bytes());
            hasher.update(&[0x1Fu8]);
            hasher.update(prev);
            let computed: Vec<u8> = hasher.finalize().to_vec();
            if &computed != entry { return Err(*idx); }
            expected_prev = entry.clone();
        }
        Ok(rows.len() as i64)
    }

    /// Recent audit entries for UI display. Returns (idx, ts_ms,
    /// category, severity, actor, action, detail, entry_hash_hex).
    pub fn audit_chain_recent(&self, limit: i64)
        -> Vec<(i64, i64, String, String, String, String, String, String)>
    {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT idx, ts_ms, category, severity, actor, action, detail, entry_hash \
             FROM security_audit_chain ORDER BY idx DESC LIMIT ?1"
        ) { Ok(s) => s, Err(_) => return Vec::new() };
        stmt.query_map(params![limit], |r| Ok((
            r.get::<_, i64>(0)?, r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?, r.get::<_, String>(3)?,
            r.get::<_, String>(4)?, r.get::<_, String>(5)?,
            r.get::<_, String>(6)?,
            {
                let bytes: Vec<u8> = r.get(7)?;
                bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>()
            },
        ))).map(|i| i.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    // ---- Capability tokens (#303) ----
    //
    // Tokens are bearer credentials granting a single named capability.
    // The raw token value is returned ONCE at issue time (caller must
    // store it); the DB only ever sees SHA-256(token). Verifying
    // consists of hashing the presented token and looking up the row.

    /// Issue a new capability token. Returns the raw token string
    /// (caller must persist it — it cannot be retrieved again) and the
    /// row id. `capability` and `label` are plaintext; `expires_at` is
    /// an optional ISO 8601 string. A cryptographically random 32-byte
    /// token (hex-encoded) is generated per call.
    pub fn issue_capability_token(
        &self,
        capability: &str,
        label: Option<&str>,
        expires_at: Option<&str>,
    ) -> Option<(String, i64)> {
        use sha2::{Sha256, Digest};
        use rand::RngCore;
        if capability.is_empty() || capability.len() > 64 { return None; }

        // 32 random bytes as hex (64 chars, no ambiguity with base64 +
        // no dependency on the base64 crate).
        let mut raw = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut raw);
        let token: String = raw.iter().map(|b| format!("{:02x}", b)).collect();
        let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();

        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let res = conn.execute(
            "INSERT INTO capability_tokens \
             (token_hash, capability, label, expires_at) \
             VALUES (?1, ?2, ?3, ?4)",
            params![hash, capability, label, expires_at],
        ).ok()?;
        if res == 0 { return None; }
        Some((token, conn.last_insert_rowid()))
    }

    /// Look up a token by its raw value. Returns the capability name on
    /// success, None if the token is unknown, revoked, or expired.
    /// Updates last_used_at + use_count atomically with the lookup.
    pub fn verify_capability_token(&self, token: &str) -> Option<String> {
        use sha2::{Sha256, Digest};
        use subtle::ConstantTimeEq;
        if token.is_empty() || token.len() > 128 { return None; }

        let presented: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        // Fetch ALL non-revoked rows and constant-time compare against each.
        // O(N) in active tokens but N is bounded (hundreds at most) and
        // the CT comparison defeats timing oracles against a direct SQL
        // WHERE token_hash = ?.
        let mut stmt = conn.prepare(
            "SELECT id, token_hash, capability, expires_at \
             FROM capability_tokens \
             WHERE revoked_at IS NULL"
        ).ok()?;
        let rows: Vec<(i64, Vec<u8>, String, Option<String>)> =
            stmt.query_map([], |r| Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))).map(|i| i.filter_map(|r| r.ok()).collect()).unwrap_or_default();
        drop(stmt);

        for (id, stored, capability, expires) in rows {
            if stored.len() != presented.len() { continue; }
            if stored.ct_eq(&presented).unwrap_u8() == 1 {
                // Check expiration (string compare works on ISO 8601).
                if let Some(exp) = expires.as_deref() {
                    let now: String = conn.query_row(
                        "SELECT datetime('now')", [],
                        |r| r.get(0),
                    ).unwrap_or_default();
                    if now.as_str() > exp { return None; }
                }
                // Bump usage counters.
                let _ = conn.execute(
                    "UPDATE capability_tokens \
                     SET last_used_at = datetime('now'), use_count = use_count + 1 \
                     WHERE id = ?1",
                    params![id],
                );
                return Some(capability);
            }
        }
        None
    }

    /// Revoke a token by its row id. Returns true if a row was updated.
    pub fn revoke_capability_token(&self, id: i64) -> bool {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE capability_tokens \
             SET revoked_at = datetime('now') WHERE id = ?1 AND revoked_at IS NULL",
            params![id],
        ).map(|n| n > 0).unwrap_or(false)
    }

    /// List active (non-revoked) tokens for display. Never returns the
    /// raw token or its hash — callers can only see metadata.
    pub fn list_capability_tokens(&self)
        -> Vec<(i64, String, Option<String>, String, Option<String>,
                Option<String>, i64)>
    {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, capability, label, issued_at, expires_at, \
                    last_used_at, use_count \
             FROM capability_tokens WHERE revoked_at IS NULL \
             ORDER BY issued_at DESC"
        ) { Ok(s) => s, Err(_) => return Vec::new() };
        stmt.query_map([], |r| Ok((
            r.get::<_, i64>(0)?, r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?, r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?, r.get::<_, Option<String>>(5)?,
            r.get::<_, i64>(6)?,
        ))).map(|i| i.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    // ---- Ingest batch registry (#326) ----

    /// Register a new ingest run. Idempotent on run_id — re-registering
    /// an existing run is a no-op.
    pub fn ingest_start(&self, run_id: &str, corpus: &str,
                         tuples_requested: i64, pid: Option<i64>) -> bool {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR IGNORE INTO ingest_batches \
             (run_id, corpus, status, tuples_requested, pid) \
             VALUES (?1, ?2, 'running', ?3, ?4)",
            params![run_id, corpus, tuples_requested, pid],
        ).map(|n| n > 0).unwrap_or(false)
    }

    /// Report progress on a running ingest.
    pub fn ingest_progress(&self, run_id: &str, ingested: i64,
                            psl_pass_rate: Option<f64>) -> bool {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE ingest_batches SET tuples_ingested = ?2, \
             psl_pass_rate = COALESCE(?3, psl_pass_rate) \
             WHERE run_id = ?1 AND status = 'running'",
            params![run_id, ingested, psl_pass_rate],
        ).map(|n| n > 0).unwrap_or(false)
    }

    /// Mark an ingest as finished. Sets status (completed / stopped /
    /// failed) and completed_at.
    pub fn ingest_finish(&self, run_id: &str, status: &str,
                          exit_reason: Option<&str>) -> bool {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE ingest_batches \
             SET status = ?2, completed_at = datetime('now'), \
                 exit_reason = ?3 \
             WHERE run_id = ?1",
            params![run_id, status, exit_reason],
        ).map(|n| n > 0).unwrap_or(false)
    }

    /// List recent ingest batches (running first, then by recency).
    pub fn ingest_list(&self, limit: i64)
        -> Vec<(String, String, String, i64, i64, Option<f64>,
                String, Option<String>, Option<String>, Option<i64>)>
    {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT run_id, corpus, status, tuples_requested, \
                    tuples_ingested, psl_pass_rate, started_at, \
                    completed_at, exit_reason, pid \
             FROM ingest_batches \
             ORDER BY CASE status WHEN 'running' THEN 0 ELSE 1 END, \
                      started_at DESC \
             LIMIT ?1"
        ) { Ok(s) => s, Err(_) => return Vec::new() };
        stmt.query_map(params![limit], |r| Ok((
            r.get::<_, String>(0)?, r.get::<_, String>(1)?,
            r.get::<_, String>(2)?, r.get::<_, i64>(3)?,
            r.get::<_, i64>(4)?, r.get::<_, Option<f64>>(5)?,
            r.get::<_, String>(6)?, r.get::<_, Option<String>>(7)?,
            r.get::<_, Option<String>>(8)?, r.get::<_, Option<i64>>(9)?,
        ))).map(|i| i.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    // ---- Drift monitor (#284) ----
    //
    // Periodic snapshots of the fact base so we can observe drift over
    // time: total count, per-source counts, FTS-indexed count, fresh
    // cnt (last 7 days), stale cnt (> 365 days), contradiction pending,
    // feedback negative rate, FSRS lapse rate.
    //
    // All queries are sampled or use indexed columns so they're cheap
    // enough to run every N minutes from a background task.

    /// Current drift snapshot. Runs sampled queries where a full scan
    /// would be expensive. Intended to be called on a schedule.
    pub fn drift_snapshot(&self) -> std::collections::BTreeMap<String, f64> {
        let mut out = std::collections::BTreeMap::new();
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        // Sampled totals — top 5000 rows by rowid DESC (free scan).
        let (fresh_sample, stale_sample, cached_sample, sample_size): (i64, i64, i64, i64) =
            conn.query_row(
                "SELECT \
                   SUM(CASE WHEN julianday('now') - julianday(COALESCE(updated_at, created_at)) <= 7 THEN 1 ELSE 0 END), \
                   SUM(CASE WHEN julianday('now') - julianday(COALESCE(updated_at, created_at)) > 365 THEN 1 ELSE 0 END), \
                   SUM(CASE WHEN hdc_vector IS NOT NULL THEN 1 ELSE 0 END), \
                   COUNT(*) \
                 FROM (SELECT updated_at, created_at, hdc_vector FROM facts ORDER BY rowid DESC LIMIT 5000)",
                [],
                |r| Ok((
                    r.get::<_, Option<i64>>(0)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(1)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    r.get::<_, i64>(3)?,
                )),
            ).unwrap_or((0, 0, 0, 0));

        let sample_f = sample_size as f64;
        if sample_f > 0.0 {
            out.insert("fresh_ratio".into(), fresh_sample as f64 / sample_f);
            out.insert("stale_ratio".into(), stale_sample as f64 / sample_f);
            out.insert("hdc_cache_ratio".into(), cached_sample as f64 / sample_f);
        }
        out.insert("sample_size".into(), sample_f);

        // Contradiction backlog — exact count on a small table.
        let pending_cons: i64 = conn.query_row(
            "SELECT COUNT(*) FROM contradictions WHERE resolved_at IS NULL",
            [], |r| r.get(0),
        ).unwrap_or(0);
        out.insert("contradictions_pending".into(), pending_cons as f64);

        // Feedback sentiment over the last 24h.
        let (pos, neg): (i64, i64) = conn.query_row(
            "SELECT \
               SUM(CASE WHEN rating = 'up' THEN 1 ELSE 0 END), \
               SUM(CASE WHEN rating IN ('down','correct') THEN 1 ELSE 0 END) \
             FROM user_feedback \
             WHERE julianday('now') - julianday(created_at) <= 1",
            [],
            |r| Ok((r.get::<_, Option<i64>>(0)?.unwrap_or(0),
                    r.get::<_, Option<i64>>(1)?.unwrap_or(0))),
        ).unwrap_or((0, 0));
        let total = pos + neg;
        out.insert("feedback_positive_24h".into(), pos as f64);
        out.insert("feedback_negative_24h".into(), neg as f64);
        out.insert(
            "feedback_negative_ratio_24h".into(),
            if total > 0 { neg as f64 / total as f64 } else { 0.0 },
        );

        // FSRS lapses over all cards (small table usually — review work).
        let (total_cards, total_lapses): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(lapses), 0) FROM fsrs_cards",
            [], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        ).unwrap_or((0, 0));
        out.insert("fsrs_total_cards".into(), total_cards as f64);
        out.insert("fsrs_total_lapses".into(), total_lapses as f64);
        out.insert(
            "fsrs_lapse_rate".into(),
            if total_cards > 0 { total_lapses as f64 / total_cards as f64 } else { 0.0 },
        );

        out
    }

    // ---- HDC vector cache per fact (#295) ----
    //
    // 10_000-dim bipolar vectors encoded via role_binding::concept_vector
    // are deterministic functions of the fact text. Re-encoding on every
    // query burns CPU; caching serialized bytes in the facts row is the
    // cheap option. Bincode gives ~1.25 kB per vector (10_000 bits + a
    // small length prefix). The column is nullable — facts without a
    // cache are simply re-encoded on demand.

    /// Store the bincode-serialized bipolar vector for a fact key.
    /// Caller is responsible for ensuring the vector corresponds to the
    /// fact's current value (call this after upsert_fact, not before).
    ///
    /// BUG ASSUMPTION: migrate() added the hdc_vector column at open
    /// time. Runtime ALTERs here would re-take the writer lock on every
    /// call and hang on large tables.
    pub fn set_fact_vector(&self, key: &str, vector_bytes: &[u8]) -> bool {
        if key.is_empty() || vector_bytes.is_empty() { return false; }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE facts SET hdc_vector = ?2 WHERE key = ?1",
            params![key, vector_bytes],
        ).map(|n| n > 0).unwrap_or(false)
    }

    /// Fetch the cached HDC vector bytes for a fact, if any.
    pub fn get_fact_vector(&self, key: &str) -> Option<Vec<u8>> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT hdc_vector FROM facts WHERE key = ?1",
            params![key],
            |r| r.get::<_, Option<Vec<u8>>>(0),
        ).ok().flatten()
    }

    /// Sample-based cache coverage for a UI badge.
    ///
    /// BUG ASSUMPTION: a full COUNT(*) on a 58M-row prod DB takes minutes,
    /// which blocks the server. We sample the most recent 5000 rows by
    /// rowid DESC (rowid is free to scan) and return
    /// (cached_in_sample, sample_size).
    ///
    /// Callers that need the exact count must query the DB directly with
    /// an offline tool (the prod UI doesn't need the precise number —
    /// a % coverage gauge is enough).
    pub fn hdc_cache_stats(&self) -> (i64, i64) {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT \
               SUM(CASE WHEN hdc_vector IS NOT NULL THEN 1 ELSE 0 END), \
               COUNT(*) \
             FROM (SELECT hdc_vector FROM facts ORDER BY rowid DESC LIMIT 5000)",
            [], |r| Ok((
                r.get::<_, Option<i64>>(0)?.unwrap_or(0),
                r.get::<_, i64>(1)?,
            )),
        ).unwrap_or((0, 0))
    }

    /// Return up to `limit` fact keys+values that do NOT yet have a cached
    /// HDC vector. Used by batch encoders to pick off uncached rows.
    ///
    /// BUG ASSUMPTION: ORDER BY on non-indexed columns (updated_at,
    /// quality_score, etc.) forces a full scan even with LIMIT. We order
    /// by rowid DESC which is free because rowid is the primary index.
    pub fn facts_without_vector(&self, limit: i64) -> Vec<(String, String)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT key, value FROM facts WHERE hdc_vector IS NULL \
             ORDER BY rowid DESC LIMIT ?1"
        ) { Ok(s) => s, Err(_) => return Vec::new() };
        stmt.query_map(params![limit], |r| Ok((
            r.get::<_, String>(0)?, r.get::<_, String>(1)?,
        ))).map(|i| i.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    /// #298 Pending contradiction count — useful for a UI badge.
    pub fn contradiction_pending_count(&self) -> i64 {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT COUNT(*) FROM contradictions WHERE resolved_at IS NULL",
            [], |r| r.get(0),
        ).unwrap_or(0)
    }

    // ---- Source trust + cross-source reconciliation (#293) ----
    //
    // Trust is the cheap, first-pass reconciliation signal. When a
    // contradiction fires at ingest (#298), `auto_resolve_contradictions`
    // walks unresolved rows and picks the value whose source has the
    // higher trust — provided the margin exceeds `min_margin`. Ties
    // (or margins below the threshold) stay unresolved for human or
    // axiom-driven review.

    /// Trust of a single source, falling back to 0.5 (unknown) when the
    /// row is missing. Never returns a negative number or NaN.
    pub fn source_trust(&self, source: &str) -> f64 {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT trust FROM source_trust WHERE source = ?1",
            params![source], |r| r.get::<_, f64>(0),
        ).ok()
         .filter(|v| v.is_finite() && *v >= 0.0 && *v <= 1.0)
         .unwrap_or(0.5)
    }

    /// Upsert a source trust row. `trust` is clamped to [0, 1] — callers
    /// passing out-of-range values get the clamped value silently.
    pub fn set_source_trust(&self, source: &str, trust: f64, notes: Option<&str>) {
        if source.is_empty() { return; }
        let clamped = trust.clamp(0.0, 1.0);
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let _ = conn.execute(
            "INSERT INTO source_trust (source, trust, notes, updated_at) \
             VALUES (?1, ?2, ?3, datetime('now')) \
             ON CONFLICT(source) DO UPDATE SET \
             trust=?2, notes=?3, updated_at=datetime('now')",
            params![source, clamped, notes],
        );
    }

    /// List all known source trust weights.
    pub fn list_source_trust(&self) -> Vec<(String, f64, Option<String>, String)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT source, trust, notes, updated_at FROM source_trust \
             ORDER BY trust DESC, source ASC"
        ) { Ok(s) => s, Err(_) => return Vec::new() };
        stmt.query_map([], |r| Ok((
            r.get::<_, String>(0)?,
            r.get::<_, f64>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, String>(3)?,
        ))).map(|i| i.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    /// Resolve every unresolved contradiction whose source-trust margin
    /// exceeds `min_margin` (default 0.2). Returns (resolved, skipped).
    ///
    /// BUG ASSUMPTION: source_trust lookups are synchronous inside the
    /// main lock; a huge contradictions table (>10k rows) will slow this
    /// down. Intended to be called manually or on a schedule, not inline
    /// with ingest.
    pub fn auto_resolve_contradictions(&self, min_margin: f64) -> (i64, i64) {
        let margin = min_margin.clamp(0.05, 0.5);
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, existing_value, incoming_value, \
                    COALESCE(existing_source, ''), COALESCE(incoming_source, '') \
             FROM contradictions WHERE resolved_at IS NULL"
        ) { Ok(s) => s, Err(_) => return (0, 0) };
        let rows: Vec<(i64, String, String, String, String)> = stmt.query_map(
            [], |r| Ok((
                r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
                r.get::<_, String>(3)?, r.get::<_, String>(4)?,
            )),
        ).map(|i| i.filter_map(|r| r.ok()).collect()).unwrap_or_default();
        drop(stmt);

        let mut resolved = 0i64;
        let mut skipped = 0i64;
        for (id, ev, iv, es, is_) in rows {
            // Inline trust lookup — we hold the conn lock already so a
            // separate `source_trust()` call would deadlock.
            let es_trust: f64 = conn.query_row(
                "SELECT trust FROM source_trust WHERE source = ?1",
                params![&es], |r| r.get(0),
            ).ok().filter(|v: &f64| v.is_finite()).unwrap_or(0.5);
            let is_trust: f64 = conn.query_row(
                "SELECT trust FROM source_trust WHERE source = ?1",
                params![&is_], |r| r.get(0),
            ).ok().filter(|v: &f64| v.is_finite()).unwrap_or(0.5);
            let diff = is_trust - es_trust;
            let chosen = if diff >= margin { Some(iv.as_str()) }
                         else if -diff >= margin { Some(ev.as_str()) }
                         else { None };
            match chosen {
                Some(val) => {
                    let _ = conn.execute(
                        "UPDATE contradictions SET resolved_at = datetime('now'), \
                         resolved_value = ?2 WHERE id = ?1",
                        params![id, val],
                    );
                    resolved += 1;
                }
                None => { skipped += 1; }
            }
        }
        (resolved, skipped)
    }

    // ---- FSRS card persistence (#337) ----
    //
    // Columns: fact_key, difficulty, stability, last_review (epoch secs),
    //          review_count, lapses, state.
    // The in-memory scheduler (cognition/fsrs_scheduler.rs) is stateless
    // across process restarts — these methods give it a backing store.

    /// Load or create the FSRS row for `fact_key`. Returns
    /// (difficulty, stability, last_review, review_count, lapses, state).
    pub fn fsrs_get_or_init(&self, fact_key: &str)
        -> (f64, f64, i64, i64, i64, String)
    {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(row) = conn.query_row(
            "SELECT difficulty, stability, last_review, review_count, lapses, state \
             FROM fsrs_cards WHERE fact_key = ?1",
            params![fact_key],
            |r| Ok((r.get::<_, f64>(0)?, r.get::<_, f64>(1)?, r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?, r.get::<_, i64>(4)?, r.get::<_, String>(5)?)),
        ) {
            return row;
        }
        // Insert default row, then return defaults.
        let _ = conn.execute(
            "INSERT OR IGNORE INTO fsrs_cards (fact_key) VALUES (?1)",
            params![fact_key],
        );
        (5.0, 0.4072, 0, 0, 0, "new".to_string())
    }

    /// Persist a card state update.
    pub fn fsrs_upsert(&self, fact_key: &str,
        difficulty: f64, stability: f64, last_review: i64,
        review_count: i64, lapses: i64, state: &str)
    {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let _ = conn.execute(
            "INSERT INTO fsrs_cards \
             (fact_key, difficulty, stability, last_review, review_count, lapses, state, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now')) \
             ON CONFLICT(fact_key) DO UPDATE SET \
             difficulty=?2, stability=?3, last_review=?4, review_count=?5, \
             lapses=?6, state=?7, updated_at=datetime('now')",
            params![fact_key, difficulty, stability, last_review,
                    review_count, lapses, state],
        );
    }

    /// Cards due for review. Uses the FSRS forgetting curve inline so the
    /// DB answers without loading every card into memory.
    ///
    /// Retrievability R(t,S) = (1 + 19/81 * t/S)^(-1/0.5).
    /// Target retention R → threshold days_due = S * (R^(-0.5) - 1) * 81/19.
    ///
    /// BUG ASSUMPTION: bundled SQLite has no POWER/pow function, so the
    /// target-retention-dependent coefficient is precomputed in Rust and
    /// passed in as a plain parameter.
    pub fn fsrs_due(&self, now_secs: i64, target_r: f64, limit: i64)
        -> Vec<(String, f64, f64, i64, i64, i64, String)>
    {
        let coeff = (target_r.powf(-0.5) - 1.0) * 81.0 / 19.0;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT fact_key, difficulty, stability, last_review, \
                    review_count, lapses, state \
             FROM fsrs_cards \
             WHERE state = 'new' \
                OR (CAST((?1 - last_review) AS REAL) / 86400.0) >= \
                   stability * ?2 \
             ORDER BY last_review ASC LIMIT ?3"
        ) { Ok(s) => s, Err(_) => return Vec::new() };
        stmt.query_map(params![now_secs, coeff, limit], |r| Ok((
            r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, f64>(2)?,
            r.get::<_, i64>(3)?, r.get::<_, i64>(4)?, r.get::<_, i64>(5)?,
            r.get::<_, String>(6)?,
        ))).map(|i| i.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    }

    /// Count total cards + due cards in one roundtrip.
    pub fn fsrs_stats(&self, now_secs: i64, target_r: f64) -> (i64, i64) {
        let coeff = (target_r.powf(-0.5) - 1.0) * 81.0 / 19.0;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let total: i64 = conn.query_row(
            "SELECT COUNT(*) FROM fsrs_cards", [], |r| r.get(0)
        ).unwrap_or(0);
        let due: i64 = conn.query_row(
            "SELECT COUNT(*) FROM fsrs_cards \
             WHERE state = 'new' \
                OR (CAST((?1 - last_review) AS REAL) / 86400.0) >= \
                   stability * ?2",
            params![now_secs, coeff], |r| r.get(0)
        ).unwrap_or(0);
        (total, due)
    }

    pub fn get_all_facts(&self) -> Vec<(String, String, String, f64)> {
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
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
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
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
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let _ = conn.execute("DELETE FROM facts WHERE key = ?1", params![key]);
    }

    /// Search facts by keyword — used for RAG (Retrieval-Augmented Generation).
    /// Uses FTS5 full-text search (52M+ rows indexed) for instant retrieval.
    /// Falls back to LIKE if FTS5 is unavailable.
    /// SUPERSOCIETY: This is what makes 52M facts useful in real-time.
    pub fn search_facts(&self, query: &str, limit: usize) -> Vec<(String, String, f64)> {
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

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
        // SECURITY: Sanitize FTS5 MATCH input — strip operators that could
        // alter query semantics (* NEAR OR NOT AND " parentheses).
        let fts_query: String = keywords.iter()
            .map(|kw| kw.chars().filter(|c| c.is_alphanumeric()).collect::<String>())
            .filter(|kw| !kw.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let mut results = Vec::new();

        // FTS5 path — BM25 ranking weighted by quality_score, bounded via
        // an inner candidate LIMIT so a common term like "water" (millions
        // of matches) doesn't force a full sort across the match set.
        //
        // REGRESSION-GUARD: without the inner LIMIT, queries containing
        // high-frequency tokens took 60-90s on the 59M-fact corpus because
        // ORDER BY rank had to score every match before the outer LIMIT
        // could kick in. Chat WS timed out and the UI went red.
        //
        // PERFORMANCE: the inner pool of 2000 candidates is sorted by
        // BM25-rank / quality and the outer LIMIT (5 typical) truncates.
        // On single-keyword common terms this is ~50-200ms instead of 60s+.
        let inner_pool: i64 = 2000;
        let mut stmt = match conn.prepare(
            "WITH candidate AS ( \
               SELECT f.key, f.value, \
                      COALESCE(f.quality_score, f.confidence, 0.5) AS q, \
                      bm25(facts_fts) AS r \
               FROM facts f JOIN facts_fts ON f.rowid = facts_fts.rowid \
               WHERE facts_fts MATCH ?1 LIMIT ?2 \
             ) \
             SELECT key, value, q FROM candidate ORDER BY r / q LIMIT ?3"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        if let Ok(rows) = stmt.query_map(params![fts_query, inner_pool, limit as i64], |row| {
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

    /// RAG retrieval with pseudo-relevance feedback (PRF) query expansion.
    ///
    /// BUG ASSUMPTION: first-pass FTS5 is recall-limited when user phrasing
    /// diverges from the fact corpus (e.g. "what's the protocol" vs. facts
    /// written as "the procedure is"). PRF mines tokens from the first-pass
    /// result pool and issues a second wider OR query, boosting rows that
    /// appeared in both passes.
    ///
    /// SECURITY: expansion tokens pass through the same alphanumeric-only
    /// filter as search_facts — FTS5 operators (* NEAR OR NOT AND " ())
    /// cannot survive the sanitiser, so there is no injection surface.
    ///
    /// AVP-PASS-1: Tier 1 existence proof — no unwrap, no panic, returns
    /// empty vec on any SQL or tokenisation failure. Fallback to narrow
    /// results when expansion yields nothing useful.
    ///
    /// UX-DEBT: opt-in via caller config until benchmark harness confirms
    /// recall gain >5pp and latency p50 regression <20ms. See task #294.
    pub fn search_facts_expanded(&self, query: &str, limit: usize) -> Vec<(String, String, f64)> {
        // SAFETY: poisoned mutex means another thread panicked while holding
        // the lock. Recover into_inner rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        let stopwords = ["the","and","for","are","but","not","you","all","can","had",
            "her","was","one","our","out","has","its","how","who","what","when",
            "where","why","this","that","with","from","they","been","have","many"];
        let is_stop = |w: &str| stopwords.contains(&w.to_lowercase().as_str());
        let alnum = |w: &str| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>();

        let keywords: Vec<String> = query.split_whitespace()
            .filter(|w| w.len() >= 3 && !is_stop(w))
            .take(5)
            .map(|w| w.to_lowercase())
            .map(|w| alnum(&w))
            .filter(|w| !w.is_empty())
            .collect();

        if keywords.is_empty() {
            return vec![];
        }

        // Pass 1 — narrow AND query. Pool used for expansion mining.
        let narrow_query: String = keywords.join(" ");
        let pool_limit: i64 = 20;
        let narrow_sql = "SELECT f.key, f.value, COALESCE(f.quality_score, f.confidence, 0.5) \
                          FROM facts f JOIN facts_fts ON f.rowid = facts_fts.rowid \
                          WHERE facts_fts MATCH ?1 \
                          ORDER BY rank / COALESCE(f.quality_score, f.confidence, 0.5) \
                          LIMIT ?2";
        let mut narrow: Vec<(String, String, f64)> = Vec::new();
        if let Ok(mut stmt) = conn.prepare(narrow_sql) {
            if let Ok(rows) = stmt.query_map(params![narrow_query, pool_limit], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?))
            }) {
                for row in rows { if let Ok(r) = row { narrow.push(r); } }
            }
        }
        if narrow.is_empty() {
            return vec![];
        }

        // Mine expansion tokens — top-5 non-stopword, non-keyword tokens from pool.
        let original: std::collections::HashSet<String> = keywords.iter().cloned().collect();
        let mut counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        for (_k, value, _s) in &narrow {
            for raw in value.split_whitespace() {
                if raw.len() < 4 { continue; }
                let lower = raw.to_lowercase();
                if is_stop(&lower) { continue; }
                let cleaned = alnum(&lower);
                if cleaned.len() < 4 { continue; }
                if original.contains(&cleaned) { continue; }
                *counts.entry(cleaned).or_insert(0) += 1;
            }
        }
        let mut ranked: Vec<(String, u32)> = counts.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        let expansion: Vec<String> = ranked.into_iter()
            .filter(|(_t, n)| *n >= 2)  // must co-occur in ≥2 pool rows
            .take(5)
            .map(|(t, _n)| t)
            .collect();

        if expansion.is_empty() {
            // Nothing to expand with — return narrow truncated.
            narrow.truncate(limit);
            return narrow;
        }

        // Pass 2 — wide OR query (originals + expansion).
        let wide_query: String = keywords.iter().chain(expansion.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join(" OR ");
        let wide_limit: i64 = (limit as i64).saturating_mul(3);
        let wide_sql = "SELECT f.key, f.value, COALESCE(f.quality_score, f.confidence, 0.5) \
                        FROM facts f JOIN facts_fts ON f.rowid = facts_fts.rowid \
                        WHERE facts_fts MATCH ?1 \
                        ORDER BY rank / COALESCE(f.quality_score, f.confidence, 0.5) \
                        LIMIT ?2";
        let mut wide: Vec<(String, String, f64)> = Vec::new();
        if let Ok(mut stmt) = conn.prepare(wide_sql) {
            if let Ok(rows) = stmt.query_map(params![wide_query, wide_limit], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, f64>(2)?))
            }) {
                for row in rows { if let Ok(r) = row { wide.push(r); } }
            }
        }

        // Merge — boost rows that appeared in both passes (high confidence signal).
        let narrow_keys: std::collections::HashSet<&str> =
            narrow.iter().map(|(k, _, _)| k.as_str()).collect();
        let mut merged: std::collections::HashMap<String, (String, f64)> =
            std::collections::HashMap::new();
        for (k, v, s) in narrow.iter() {
            merged.insert(k.clone(), (v.clone(), s * 1.3));
        }
        for (k, v, s) in wide.into_iter() {
            let boosted = if narrow_keys.contains(k.as_str()) { s * 1.3 } else { s };
            merged.entry(k).and_modify(|e| {
                if boosted > e.1 { e.1 = boosted; }
            }).or_insert((v, boosted));
        }

        let mut out: Vec<(String, String, f64)> = merged.into_iter()
            .map(|(k, (v, s))| (k, v, s))
            .collect();
        // Quality-weighted score is "smaller is better" (rank/quality). Keep
        // sort direction identical to search_facts ordering.
        out.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        out.truncate(limit);
        out
    }

    // ---- Conversations ----

    pub fn save_conversation(&self, id: &str, title: &str, pinned: bool, starred: bool) {
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
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
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "INSERT INTO messages (conversation_id, role, content, timestamp, meta_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![convo_id, role, content, timestamp, meta],
        ) {
            warn!("// PERSISTENCE: save_message failed: {}", e);
        }
    }

    pub fn get_conversations(&self) -> Vec<(String, String, bool, bool, String)> {
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
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
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
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
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let _ = conn.execute("DELETE FROM messages WHERE conversation_id = ?1", params![id]);
        let _ = conn.execute("DELETE FROM conversations WHERE id = ?1", params![id]);
    }

    // ---- Training results ----

    pub fn log_training_result(&self, domain: &str, accuracy: f64, total: usize, correct: usize) {
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "INSERT INTO training_results (domain, accuracy, total, correct) VALUES (?1, ?2, ?3, ?4)",
            params![domain, accuracy, total as i64, correct as i64],
        ) {
            warn!("// PERSISTENCE: log_training_result failed: {}", e);
        }
    }

    pub fn get_training_history(&self, limit: usize) -> Vec<(String, f64, i64, i64, String)> {
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
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
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let _ = conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=?2",
            params![key, value],
        );
    }

    pub fn get_setting(&self, key: &str) -> Option<String> {
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
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
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let _ = conn.execute(
            "INSERT OR REPLACE INTO user_profile (key, value, category, learned_at) \
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![key, value, category],
        );
    }

    /// Load all user profile facts. Called on startup to hydrate the agent's
    /// understanding of the user. Returns (key, value, category) tuples.
    pub fn load_profile(&self) -> Vec<(String, String, String)> {
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
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
        // SAFETY: poisoned mutex means another thread panicked while holding the lock.
        // We recover or return defaults rather than propagating the panic.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT value FROM user_profile WHERE key = ?1",
            params![key],
            |row| row.get(0),
        ).ok()
    }

    // ---- Knowledge Graph: Fact Edges ----

    /// Create an edge between two facts.
    /// BUG ASSUMPTION: source_key and target_key may not exist in facts table.
    /// We allow dangling edges to support lazy graph construction.
    pub fn create_edge(&self, source_key: &str, target_key: &str, edge_type: &str, strength: f64, evidence: Option<&str>) {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "INSERT OR REPLACE INTO fact_edges (source_key, target_key, edge_type, strength, evidence) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![source_key, target_key, edge_type, strength, evidence],
        ) {
            warn!("// PERSISTENCE: create_edge failed: {}", e);
        }
    }

    /// Get all edges originating from a fact (outbound connections).
    pub fn get_edges_from(&self, source_key: &str) -> Vec<(String, String, f64, Option<String>)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT target_key, edge_type, strength, evidence FROM fact_edges WHERE source_key = ?1 ORDER BY strength DESC"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(params![source_key], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        }).unwrap_or_else(|_| panic!("query_map failed")).filter_map(|r| r.ok()).collect()
    }

    /// Get all edges pointing to a fact (inbound connections).
    pub fn get_edges_to(&self, target_key: &str) -> Vec<(String, String, f64, Option<String>)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT source_key, edge_type, strength, evidence FROM fact_edges WHERE target_key = ?1 ORDER BY strength DESC"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(params![target_key], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        }).unwrap_or_else(|_| panic!("query_map failed")).filter_map(|r| r.ok()).collect()
    }

    /// Get all neighbors of a fact (both inbound and outbound).
    pub fn get_neighbors(&self, fact_key: &str, limit: usize) -> Vec<(String, String, f64, String)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT CASE WHEN source_key = ?1 THEN target_key ELSE source_key END as neighbor, \
                    edge_type, strength, \
                    CASE WHEN source_key = ?1 THEN 'outbound' ELSE 'inbound' END as direction \
             FROM fact_edges \
             WHERE source_key = ?1 OR target_key = ?1 \
             ORDER BY strength DESC LIMIT ?2"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(params![fact_key, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
            ))
        }).unwrap_or_else(|_| panic!("query_map failed")).filter_map(|r| r.ok()).collect()
    }

    /// Count total edges in the graph.
    pub fn count_edges(&self) -> i64 {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row("SELECT COUNT(*) FROM fact_edges", [], |row| row.get(0))
            .unwrap_or(0)
    }

    /// Summarise the causal / taxonomic neighbourhood of a concept for
    /// chat responses (#336). Looks up the `concept:<normalized>` key
    /// in fact_edges and groups results by edge_type.
    ///
    /// Returns None if the concept has no edges. Non-empty results are
    /// shaped as:
    ///
    ///   "structured context for X:
    ///     is a: cat, pet, living thing
    ///     causes: purring, allergies
    ///     has prerequisite: food, water"
    ///
    /// `per_type_limit` caps how many neighbours appear per predicate.
    pub fn causal_summary(&self, concept: &str, per_type_limit: usize) -> Option<String> {
        let norm: String = concept.trim().to_lowercase().split_whitespace()
            .collect::<Vec<_>>().join(" ");
        if norm.is_empty() { return None; }
        let concept_key = format!("concept:{}", norm);

        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        // Outbound: "X --IsA--> category", "X --Causes--> effect", etc.
        let mut stmt = match conn.prepare(
            "SELECT edge_type, target_key, strength FROM fact_edges \
             WHERE source_key = ?1 ORDER BY strength DESC LIMIT 200"
        ) { Ok(s) => s, Err(_) => return None };
        let outbound: Vec<(String, String, f64)> = stmt.query_map(
            rusqlite::params![&concept_key], |r| Ok((
                r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, f64>(2)?,
            ))).map(|iter| iter.filter_map(|x| x.ok()).collect()).unwrap_or_default();

        // Inbound for reverse questions: "what causes X?"
        // Only "Causes" + "HasPrerequisite" + "MotivatedByGoal" give useful
        // reverse-direction answers; skip IsA (too many sub-classes).
        let mut stmt2 = match conn.prepare(
            "SELECT edge_type, source_key, strength FROM fact_edges \
             WHERE target_key = ?1 \
               AND edge_type IN ('Causes','HasPrerequisite','MotivatedByGoal','CausesDesire') \
             ORDER BY strength DESC LIMIT 80"
        ) { Ok(s) => s, Err(_) => return None };
        let inbound: Vec<(String, String, f64)> = stmt2.query_map(
            rusqlite::params![&concept_key], |r| Ok((
                r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, f64>(2)?,
            ))).map(|iter| iter.filter_map(|x| x.ok()).collect()).unwrap_or_default();

        if outbound.is_empty() && inbound.is_empty() { return None; }

        use std::collections::BTreeMap;
        let mut out_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (et, t, _s) in &outbound {
            let tgt = t.strip_prefix("concept:").unwrap_or(t).to_string();
            out_groups.entry(et.clone()).or_default().push(tgt);
        }
        let mut in_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (et, s, _w) in &inbound {
            let src = s.strip_prefix("concept:").unwrap_or(s).to_string();
            in_groups.entry(et.clone()).or_default().push(src);
        }

        // Format — use human-readable predicate labels.
        fn forward_label(et: &str) -> String {
            match et {
                "IsA" => "is a".into(),
                "UsedFor" => "used for".into(),
                "Causes" => "causes".into(),
                "HasPrerequisite" => "requires".into(),
                "HasSubevent" => "involves".into(),
                "PartOf" => "part of".into(),
                "MotivatedByGoal" => "motivated by".into(),
                "CausesDesire" => "makes one want".into(),
                other => other.to_string(),
            }
        }
        fn reverse_label(et: &str) -> String {
            match et {
                "Causes" => "can be caused by".into(),
                "HasPrerequisite" => "is a prerequisite for".into(),
                "MotivatedByGoal" => "motivates".into(),
                "CausesDesire" => "wanted by someone doing".into(),
                other => other.to_string(),
            }
        }

        let mut out = format!("Structured context for \"{}\":\n", norm);
        for (et, mut xs) in out_groups {
            xs.truncate(per_type_limit);
            out.push_str(&format!("- {}: {}\n", forward_label(&et), xs.join(", ")));
        }
        for (et, mut xs) in in_groups {
            xs.truncate(per_type_limit);
            out.push_str(&format!("- {}: {}\n", reverse_label(&et), xs.join(", ")));
        }
        Some(out)
    }

    /// Get edge type distribution.
    pub fn edge_type_stats(&self) -> Vec<(String, i64)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT edge_type, COUNT(*) FROM fact_edges GROUP BY edge_type ORDER BY COUNT(*) DESC"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }).unwrap_or_else(|_| panic!("query_map failed")).filter_map(|r| r.ok()).collect()
    }

    // ---- Knowledge Graph: Domain Cross-References ----

    /// Record a cross-domain concept link.
    pub fn create_domain_xref(&self, domain_a: &str, domain_b: &str, concept: &str, strength: f64, example_key: Option<&str>) {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        // Normalize order: always store (min, max) to avoid duplicates
        let (da, db) = if domain_a < domain_b { (domain_a, domain_b) } else { (domain_b, domain_a) };
        if let Err(e) = conn.execute(
            "INSERT OR REPLACE INTO domain_xref (domain_a, domain_b, shared_concept, strength, example_fact_key) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![da, db, concept, strength, example_key],
        ) {
            warn!("// PERSISTENCE: create_domain_xref failed: {}", e);
        }
    }

    // ---- Fact Versioning ----

    /// Record a fact version change. Called automatically by upsert_fact when a fact
    /// is updated, and by quality score changes.
    /// BUG ASSUMPTION: fact_key should correspond to an existing fact, but we don't enforce this
    /// to allow pre-creation version records.
    pub fn record_version(&self, fact_key: &str, old_value: Option<&str>, new_value: &str,
                          old_quality: Option<f64>, new_quality: Option<f64>,
                          change_type: &str, changed_by: &str, reason: Option<&str>) {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "INSERT INTO fact_versions (fact_key, old_value, new_value, old_quality, new_quality, \
             change_type, changed_by, reason) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![fact_key, old_value, new_value, old_quality, new_quality, change_type, changed_by, reason],
        ) {
            warn!("// PERSISTENCE: record_version failed: {}", e);
        }
    }

    /// Get version history for a specific fact.
    pub fn get_fact_history(&self, fact_key: &str, limit: usize) -> Vec<(String, Option<String>, String, Option<f64>, Option<f64>, String, String, String)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT fact_key, old_value, new_value, old_quality, new_quality, \
             change_type, changed_by, created_at \
             FROM fact_versions WHERE fact_key = ?1 ORDER BY id DESC LIMIT ?2"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(params![fact_key, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        }).unwrap_or_else(|_| panic!("query_map failed")).filter_map(|r| r.ok()).collect()
    }

    /// Get recent version changes across all facts.
    pub fn get_recent_versions(&self, limit: usize) -> Vec<(String, String, String, String)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT fact_key, change_type, changed_by, created_at \
             FROM fact_versions ORDER BY id DESC LIMIT ?1"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        }).unwrap_or_else(|_| panic!("query_map failed")).filter_map(|r| r.ok()).collect()
    }

    /// Count total version records.
    pub fn count_versions(&self) -> i64 {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row("SELECT COUNT(*) FROM fact_versions", [], |row| row.get(0)).unwrap_or(0)
    }

    /// Get version change type distribution.
    pub fn version_stats(&self) -> Vec<(String, i64)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT change_type, COUNT(*) FROM fact_versions GROUP BY change_type ORDER BY COUNT(*) DESC"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }).unwrap_or_else(|_| panic!("query_map failed")).filter_map(|r| r.ok()).collect()
    }

    // ---- Audit Log ----

    /// Log an audit pass result.
    pub fn log_audit(&self, audit_type: &str, pass_number: i32, tier: i32, status: &str,
                     findings_total: i32, findings_fixed: i32, score: Option<f64>, details: Option<&str>) {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "INSERT INTO audit_log (audit_type, pass_number, tier, status, findings_total, \
             findings_fixed, findings_open, score, details, completed_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
            params![audit_type, pass_number, tier, status, findings_total, findings_fixed,
                    findings_total - findings_fixed, score, details],
        ) {
            warn!("// PERSISTENCE: log_audit failed: {}", e);
        }
    }

    /// Get audit history, most recent first.
    pub fn get_audit_history(&self, limit: usize) -> Vec<(i64, String, i32, i32, String, i32, i32, i32, Option<f64>, String)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, audit_type, pass_number, tier, status, findings_total, \
             findings_fixed, findings_open, score, COALESCE(completed_at, started_at) \
             FROM audit_log ORDER BY id DESC LIMIT ?1"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(params![limit as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, i32>(6)?,
                row.get::<_, i32>(7)?,
                row.get::<_, Option<f64>>(8)?,
                row.get::<_, String>(9)?,
            ))
        }).unwrap_or_else(|_| panic!("query_map failed")).filter_map(|r| r.ok()).collect()
    }

    /// Get compliance scorecard: tier completion stats.
    pub fn get_compliance_scorecard(&self) -> Vec<(i32, i64, i64, f64)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT tier, COUNT(*) as total_passes, \
             SUM(CASE WHEN status='passed' THEN 1 ELSE 0 END) as passed, \
             AVG(COALESCE(score, 0)) as avg_score \
             FROM audit_log GROUP BY tier ORDER BY tier"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        }).unwrap_or_else(|_| panic!("query_map failed")).filter_map(|r| r.ok()).collect()
    }

    // ---- Training Provenance ----

    /// Log a training run's provenance — which data was used and what changed.
    pub fn log_training_provenance(&self, run_id: &str, source: Option<&str>, domain: Option<&str>,
                                    pairs: i32, before: Option<f64>, after: Option<f64>,
                                    model: Option<&str>, notes: Option<&str>) {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let delta = match (before, after) {
            (Some(b), Some(a)) => Some(a - b),
            _ => None,
        };
        if let Err(e) = conn.execute(
            "INSERT INTO training_provenance (run_id, source_file, domain, pairs_used, \
             accuracy_before, accuracy_after, accuracy_delta, model, notes) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![run_id, source, domain, pairs, before, after, delta, model, notes],
        ) {
            warn!("// PERSISTENCE: log_training_provenance failed: {}", e);
        }
    }

    /// Get training provenance history.
    pub fn get_training_provenance(&self, limit: usize) -> Vec<(String, Option<String>, Option<String>, i32, Option<f64>, Option<f64>, String)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT run_id, source_file, domain, pairs_used, accuracy_before, accuracy_after, created_at \
             FROM training_provenance ORDER BY id DESC LIMIT ?1"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, Option<f64>>(5)?,
                row.get::<_, String>(6)?,
            ))
        }).unwrap_or_else(|_| panic!("query_map")).filter_map(|r| r.ok()).collect()
    }

    /// Get all cross-references for a domain.
    pub fn get_domain_xrefs(&self, domain: &str) -> Vec<(String, String, f64, Option<String>)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT CASE WHEN domain_a = ?1 THEN domain_b ELSE domain_a END as other_domain, \
                    shared_concept, strength, example_fact_key \
             FROM domain_xref \
             WHERE domain_a = ?1 OR domain_b = ?1 \
             ORDER BY strength DESC"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(params![domain], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        }).unwrap_or_else(|_| panic!("query_map failed")).filter_map(|r| r.ok()).collect()
    }

    /// Get all domain cross-references as a full adjacency list.
    pub fn get_all_domain_xrefs(&self) -> Vec<(String, String, String, f64)> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT domain_a, domain_b, shared_concept, strength \
             FROM domain_xref ORDER BY strength DESC LIMIT 500"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
            ))
        }).unwrap_or_else(|_| panic!("query_map failed")).filter_map(|r| r.ok()).collect()
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
    fn contradiction_logged_when_high_conf_values_disagree() {
        // #298: existing 0.9 "8" vs incoming 0.95 "9" → logged
        let db = temp_db();
        db.upsert_fact("planet_count", "8", "astronomy_a", 0.9);
        db.upsert_fact("planet_count", "9", "astronomy_b", 0.95);
        let rows = db.recent_contradictions(10, true);
        assert_eq!(rows.len(), 1, "expected one contradiction row");
        assert_eq!(rows[0].1, "planet_count");
        assert_eq!(rows[0].2, "8"); // existing_value
        assert_eq!(rows[0].3, "9"); // incoming_value
        assert_eq!(db.contradiction_pending_count(), 1);
    }

    #[test]
    fn contradiction_skipped_when_same_value() {
        // Trivial re-ingest of the same value is never a contradiction.
        let db = temp_db();
        db.upsert_fact("role", "architect", "user", 0.9);
        db.upsert_fact("role", "architect", "other_user", 0.95);
        assert_eq!(db.contradiction_pending_count(), 0);
    }

    #[test]
    fn contradiction_skipped_when_existing_low_conf() {
        // Existing < 0.7 → no contradiction, incoming just wins.
        let db = temp_db();
        db.upsert_fact("k", "a", "src1", 0.5);
        db.upsert_fact("k", "b", "src2", 0.95);
        assert_eq!(db.contradiction_pending_count(), 0);
    }

    #[test]
    fn contradiction_skipped_when_incoming_low_conf() {
        // Incoming < 0.7 → no contradiction, the incoming is too weak to
        // count as a disagreement against high-confidence existing data.
        let db = temp_db();
        db.upsert_fact("k", "a", "src1", 0.95);
        db.upsert_fact("k", "b", "src2", 0.4);
        assert_eq!(db.contradiction_pending_count(), 0);
    }

    #[test]
    fn capability_token_issue_and_verify() {
        let db = temp_db();
        let (token, id) = db.issue_capability_token("ingest", Some("python_script"), None)
            .expect("issue failed");
        assert!(!token.is_empty());
        assert!(id > 0);
        assert_eq!(db.verify_capability_token(&token), Some("ingest".to_string()));
    }

    #[test]
    fn capability_token_unknown_rejected() {
        let db = temp_db();
        assert_eq!(db.verify_capability_token("totally-made-up-token"), None);
    }

    #[test]
    fn capability_token_revoke_blocks_verification() {
        let db = temp_db();
        let (token, id) = db.issue_capability_token("admin", None, None).unwrap();
        assert_eq!(db.verify_capability_token(&token), Some("admin".to_string()));
        assert!(db.revoke_capability_token(id));
        assert_eq!(db.verify_capability_token(&token), None);
    }

    #[test]
    fn capability_token_expired_rejected() {
        let db = temp_db();
        let (token, _id) = db.issue_capability_token(
            "scope", None, Some("2020-01-01 00:00:00")
        ).unwrap();
        assert_eq!(db.verify_capability_token(&token), None);
    }

    #[test]
    fn capability_token_list_hides_hashes() {
        let db = temp_db();
        let (_, _) = db.issue_capability_token("ingest", Some("lbl1"), None).unwrap();
        let (_, _) = db.issue_capability_token("ingest", Some("lbl2"), None).unwrap();
        let rows = db.list_capability_tokens();
        assert_eq!(rows.len(), 2);
        // Fields: (id, cap, label, issued, expires, last_used, uses).
        // Note there is no hash or raw token in the tuple.
        assert!(rows.iter().all(|r| r.1 == "ingest"));
    }

    #[test]
    fn ingest_lifecycle_roundtrip() {
        let db = temp_db();
        assert!(db.ingest_start("run-abc", "conceptnet", 5000, Some(12345)));
        // Idempotent: second start on same run_id is a no-op.
        assert!(!db.ingest_start("run-abc", "conceptnet", 5000, Some(12345)));
        assert!(db.ingest_progress("run-abc", 1234, Some(0.87)));
        assert!(db.ingest_finish("run-abc", "completed", Some("reached quota")));
        let list = db.ingest_list(10);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "run-abc");
        assert_eq!(list[0].2, "completed");
        assert_eq!(list[0].4, 1234);
        assert!((list[0].5.unwrap_or(0.0) - 0.87).abs() < 1e-9);
    }

    #[test]
    fn ingest_list_sorts_running_first() {
        let db = temp_db();
        db.ingest_start("old", "c1", 100, None);
        db.ingest_finish("old", "completed", None);
        // Small delay not needed — running-first ordering is by status,
        // not by started_at ordering tie-break.
        db.ingest_start("new_running", "c2", 200, None);
        let list = db.ingest_list(10);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].0, "new_running");
        assert_eq!(list[0].2, "running");
        assert_eq!(list[1].0, "old");
    }

    #[test]
    fn audit_chain_append_and_verify() {
        let db = temp_db();
        let (id1, h1) = db.audit_chain_append(
            "auth", "Info", "tester", "auth_success", "first event"
        ).unwrap();
        let (id2, h2) = db.audit_chain_append(
            "auth", "Info", "tester", "auth_success", "second event"
        ).unwrap();
        assert!(id2 > id1);
        assert_ne!(h1, h2);
        assert_eq!(db.audit_chain_verify(), Ok(2));
        assert_eq!(db.audit_chain_recent(10).len(), 2);
    }

    #[test]
    fn audit_chain_detects_tampering() {
        // Write two rows, then mutate row 1's detail and expect verify to fail.
        let db = temp_db();
        db.audit_chain_append("cat", "Info", "a", "x", "row one").unwrap();
        db.audit_chain_append("cat", "Info", "a", "x", "row two").unwrap();
        {
            let conn = db.conn.lock().unwrap_or_else(|e| e.into_inner());
            conn.execute(
                "UPDATE security_audit_chain SET detail='TAMPERED' WHERE idx = 1",
                [],
            ).unwrap();
        }
        match db.audit_chain_verify() {
            Err(idx) => assert_eq!(idx, 1),
            Ok(n) => panic!("tamper not detected, verified {} rows", n),
        }
    }

    #[test]
    fn audit_chain_empty_is_valid() {
        let db = temp_db();
        assert_eq!(db.audit_chain_verify(), Ok(0));
    }

    #[test]
    fn hdc_vector_roundtrip() {
        let db = temp_db();
        db.upsert_fact("concept:water", "H2O is a compound", "test", 0.9);
        let payload = vec![0xAB, 0xCD, 0xEF, 0x12, 0x34];
        assert!(db.set_fact_vector("concept:water", &payload));
        assert_eq!(db.get_fact_vector("concept:water"), Some(payload));
    }

    #[test]
    fn hdc_cache_stats_counts_sample_correctly() {
        let db = temp_db();
        db.upsert_fact("a", "x", "test", 0.9);
        db.upsert_fact("b", "y", "test", 0.9);
        db.upsert_fact("c", "z", "test", 0.9);
        db.set_fact_vector("a", &[1, 2, 3]);
        let (cached_sample, sample_size) = db.hdc_cache_stats();
        assert_eq!(cached_sample, 1);
        assert_eq!(sample_size, 3);
    }

    #[test]
    fn hdc_facts_without_vector_lists_uncached() {
        let db = temp_db();
        db.upsert_fact("cached", "v", "test", 0.9);
        db.upsert_fact("uncached", "v", "test", 0.9);
        db.set_fact_vector("cached", &[1]);
        let rows = db.facts_without_vector(10);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "uncached");
    }

    #[test]
    fn source_trust_default_is_half() {
        let db = temp_db();
        assert!((db.source_trust("nonexistent") - 0.5).abs() < 1e-9);
    }

    #[test]
    fn source_trust_set_and_get() {
        let db = temp_db();
        db.set_source_trust("wikidata", 0.85, Some("vetted"));
        assert!((db.source_trust("wikidata") - 0.85).abs() < 1e-9);
        db.set_source_trust("wikidata", 2.5, None); // clamped
        assert!((db.source_trust("wikidata") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn source_trust_listing_sorted_desc() {
        let db = temp_db();
        db.set_source_trust("low", 0.1, None);
        db.set_source_trust("high", 0.9, None);
        db.set_source_trust("mid", 0.5, None);
        let rows = db.list_source_trust();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].0, "high");
        assert_eq!(rows[2].0, "low");
    }

    #[test]
    fn auto_resolve_prefers_high_trust() {
        let db = temp_db();
        db.upsert_fact("planet_count", "8", "astronomy_solid", 0.9);
        db.upsert_fact("planet_count", "9", "astronomy_spam", 0.95);
        db.set_source_trust("astronomy_solid", 0.9, None);
        db.set_source_trust("astronomy_spam", 0.3, None);
        let (resolved, _skipped) = db.auto_resolve_contradictions(0.2);
        assert_eq!(resolved, 1);
        let all = db.recent_contradictions(10, false);
        assert_eq!(all.len(), 1);
        // existing source has higher trust → resolved_value = existing "8"
        assert_eq!(all[0].10.as_deref(), Some("8"));
    }

    #[test]
    fn auto_resolve_skips_similar_trust() {
        let db = temp_db();
        db.upsert_fact("k", "a", "src1", 0.9);
        db.upsert_fact("k", "b", "src2", 0.9);
        db.set_source_trust("src1", 0.6, None);
        db.set_source_trust("src2", 0.65, None); // 0.05 margin
        let (resolved, skipped) = db.auto_resolve_contradictions(0.2);
        assert_eq!(resolved, 0);
        assert_eq!(skipped, 1);
        assert_eq!(db.contradiction_pending_count(), 1);
    }

    #[test]
    fn auto_resolve_uses_default_trust_for_unknown_sources() {
        // Both sources at default 0.5 → no margin → skipped.
        let db = temp_db();
        db.upsert_fact("k", "a", "unknown1", 0.9);
        db.upsert_fact("k", "b", "unknown2", 0.9);
        let (resolved, skipped) = db.auto_resolve_contradictions(0.2);
        assert_eq!(resolved, 0);
        assert_eq!(skipped, 1);
    }

    #[test]
    fn fsrs_default_card_is_due() {
        let db = temp_db();
        let (d, s, lr, rc, _l, state) = db.fsrs_get_or_init("concept:volcano");
        assert_eq!((d, lr, rc), (5.0, 0, 0));
        assert!(s > 0.0);
        assert_eq!(state, "new");
        let due = db.fsrs_due(0, 0.9, 10);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].0, "concept:volcano");
    }

    #[test]
    fn fsrs_good_review_raises_stability() {
        let db = temp_db();
        let (d, s0, _, rc, _l, _st) = db.fsrs_get_or_init("k");
        // Rating 3 = Good → stability ×2.5
        db.fsrs_upsert("k", d - 0.15, s0 * 2.5, 86400, rc + 1, 0, "review");
        let (_, s1, _, _, _, _) = db.fsrs_get_or_init("k");
        assert!((s1 - s0 * 2.5).abs() < 1e-9);
    }

    #[test]
    fn fsrs_reviewed_card_not_immediately_due() {
        // A card reviewed today with stability=10 should not be due
        // against target_r=0.9 until ~15 days later.
        let db = temp_db();
        db.fsrs_upsert("k", 5.0, 10.0, 100_000, 1, 0, "review");
        let due_now = db.fsrs_due(100_000, 0.9, 10);
        assert_eq!(due_now.len(), 0, "fresh review should not be due");
        // 30 days later it should be due.
        let due_later = db.fsrs_due(100_000 + 30 * 86400, 0.9, 10);
        assert_eq!(due_later.len(), 1);
    }

    #[test]
    fn contradiction_resolve_round_trip() {
        let db = temp_db();
        db.upsert_fact("k", "a", "src1", 0.9);
        db.upsert_fact("k", "b", "src2", 0.9);
        let rows = db.recent_contradictions(10, true);
        assert_eq!(rows.len(), 1);
        let id = rows[0].0;
        assert!(db.resolve_contradiction(id, "a"));
        assert_eq!(db.contradiction_pending_count(), 0);
        // Resolved row is still visible when only_unresolved=false.
        let all = db.recent_contradictions(10, false);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].10.as_deref(), Some("a")); // resolved_value
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

    #[test]
    fn fact_versioning() {
        let db = temp_db();
        db.record_version("test_fact", None, "initial value", None, Some(0.8),
                          "created", "system", Some("initial import"));
        db.record_version("test_fact", Some("initial value"), "updated value",
                          Some(0.8), Some(0.9), "updated", "user", Some("correction"));

        let history = db.get_fact_history("test_fact", 10);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].5, "updated"); // most recent first

        let recent = db.get_recent_versions(10);
        assert_eq!(recent.len(), 2);

        assert_eq!(db.count_versions(), 2);

        let stats = db.version_stats();
        assert!(stats.len() >= 2);
    }

    #[test]
    fn edge_roundtrip() {
        let db = temp_db();
        db.create_edge("fact_a", "fact_b", "related", 0.8, Some("shared topic"));
        db.create_edge("fact_a", "fact_c", "supports", 0.9, None);

        let from = db.get_edges_from("fact_a");
        assert_eq!(from.len(), 2);

        let to = db.get_edges_to("fact_b");
        assert_eq!(to.len(), 1);

        let neighbors = db.get_neighbors("fact_a", 10);
        assert_eq!(neighbors.len(), 2);

        assert_eq!(db.count_edges(), 2);
    }

    #[test]
    fn domain_xref() {
        let db = temp_db();
        db.create_domain_xref("cybersecurity", "mathematics", "encryption",
                              0.8, Some("rsa_fact_1"));
        let xrefs = db.get_domain_xrefs("cybersecurity");
        assert_eq!(xrefs.len(), 1);
        assert_eq!(xrefs[0].0, "mathematics");

        let all = db.get_all_domain_xrefs();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn version_trigger_auto_records_on_update() {
        let db = temp_db();
        db.upsert_fact("planet_count", "8", "astronomy", 0.9);
        // No history yet — just the initial insert.
        assert_eq!(db.count_versions(), 0);

        // Update value — trigger must record the pre-image.
        db.upsert_fact("planet_count", "9", "astronomy", 0.9);
        assert_eq!(db.count_versions(), 1, "value change should record");
        let hist = db.get_fact_history("planet_count", 10);
        assert_eq!(hist.len(), 1);
        // hist row shape: (id, old_value, new_value, old_q, new_q, change_type, ...)
        assert_eq!(hist[0].1.as_deref(), Some("8"));
        assert_eq!(hist[0].2, "9");

        // Confidence-only change also records.
        db.upsert_fact("planet_count", "9", "astronomy", 0.7);
        assert_eq!(db.count_versions(), 2, "confidence change should record");

        // No-op update (same value + same confidence) should NOT record.
        db.upsert_fact("planet_count", "9", "astronomy", 0.7);
        assert_eq!(db.count_versions(), 2, "no-op update must not record");
    }

    #[test]
    fn search_expanded_empty_query() {
        let db = temp_db();
        db.upsert_fact("k1", "ocean floor mapping advances", "science", 0.9);
        assert!(db.search_facts_expanded("", 10).is_empty());
        assert!(db.search_facts_expanded("   ", 10).is_empty());
    }

    #[test]
    fn search_expanded_no_match() {
        let db = temp_db();
        db.upsert_fact("k1", "sheep graze on pasture", "farm", 0.9);
        assert!(db.search_facts_expanded("quantum entanglement photon", 10).is_empty());
    }

    #[test]
    fn search_expanded_falls_back_when_nothing_to_expand() {
        // #321: previously failed because migrate() didn't expose
        // quality_score, so COALESCE(f.quality_score, ...) broke the
        // prepared statement on fresh DBs. Idempotent ALTER in migrate()
        // fixes the column parity with prod.
        let db = temp_db();
        db.upsert_fact("k1", "unique phrase xylophone", "test", 0.9);
        let baseline = db.search_facts("xylophone", 10);
        assert_eq!(baseline.len(), 1, "search_facts baseline broken — FTS5 not wired");
        let out = db.search_facts_expanded("xylophone", 10);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, "k1");
    }

    #[test]
    fn search_expanded_widens_recall() {
        let db = temp_db();
        // Build a pool where "protocol" co-occurs with "handshake" and "packet"
        for i in 0..5 {
            db.upsert_fact(
                &format!("proto_{}", i),
                "protocol handshake packet exchange",
                "networking", 0.9,
            );
        }
        // Divergent-phrasing fact that uses expansion terms but not the query
        db.upsert_fact(
            "divergent",
            "the handshake establishes packet ordering",
            "networking", 0.9,
        );

        let narrow = db.search_facts("protocol", 20);
        let expanded = db.search_facts_expanded("protocol", 20);

        // Expansion must at least preserve narrow recall
        assert!(expanded.len() >= narrow.len(),
                "expansion regressed recall: narrow={} expanded={}",
                narrow.len(), expanded.len());
        // AVP-PASS-1: result is bounded by requested limit
        assert!(expanded.len() <= 20);
    }
}
