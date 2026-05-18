// === C10 Agent Runtime: Local State Database (SQLite) ===
//
// Thread-local agent persistent state: todos, knowledge, handoffs,
// message log, failure log, and key-value state store.
//
// Source: docs/04-specifications/runtime/agent-runtime-spec.md Section 6

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};

use crate::types::{
    FailureRecord, HandoffRecord, Hash32, KnowledgeEntry, KnowledgeKind, TodoItem, TodoStatus,
};

/// Wraps a `rusqlite::Connection` for agent-local persistent state.
pub struct Database {
    conn: Connection,
}

// ── Open / Migrate ──

impl Database {
    /// Opens SQLite at `path`, enables WAL mode, and runs migrations.
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Creates all tables if they do not already exist.
    pub fn migrate(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS todos (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                status TEXT NOT NULL,
                context TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS knowledge (
                id BLOB PRIMARY KEY,
                kind TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                last_read_at INTEGER NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS handoffs (
                session_id BLOB PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                summary BLOB NOT NULL,
                next_actions_json TEXT NOT NULL,
                todos_snapshot_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS failures (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                tool_name TEXT NOT NULL,
                input_hash BLOB NOT NULL,
                error_kind TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;
        Ok(())
    }
}

// ── Todo CRUD ──

impl Database {
    pub fn insert_todo(&self, item: &TodoItem) -> Result<(), rusqlite::Error> {
        let now = now_secs();
        self.conn.execute(
            "INSERT INTO todos (id, content, status, context, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![item.id, item.content, todo_status_str(item.status), item.context, now, now,],
        )?;
        Ok(())
    }

    pub fn update_todo_status(
        &self,
        id: &str,
        status: TodoStatus,
        context_update: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let now = now_secs();
        if let Some(ctx) = context_update {
            self.conn.execute(
                "UPDATE todos SET status = ?1, context = ?2, updated_at = ?3 WHERE id = ?4",
                params![todo_status_str(status), ctx, now, id],
            )?;
        } else {
            self.conn.execute(
                "UPDATE todos SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![todo_status_str(status), now, id],
            )?;
        }
        Ok(())
    }

    /// Returns all todos whose status is not `Done` or `Cancelled`.
    pub fn get_active_todos(&self) -> Result<Vec<TodoItem>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, status, context FROM todos
             WHERE status NOT IN ('Done', 'Cancelled')
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TodoItem {
                id: row.get(0)?,
                content: row.get(1)?,
                status: parse_todo_status(&row.get::<_, String>(2)?),
                context: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_todo(&self, id: &str) -> Result<Option<TodoItem>, rusqlite::Error> {
        let mut stmt =
            self.conn.prepare("SELECT id, content, status, context FROM todos WHERE id = ?1")?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(TodoItem {
                id: row.get(0)?,
                content: row.get(1)?,
                status: parse_todo_status(&row.get::<_, String>(2)?),
                context: row.get(3)?,
            })
        })?;
        match rows.next() {
            Some(Ok(item)) => Ok(Some(item)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }
}

// ── Knowledge CRUD ──

impl Database {
    pub fn insert_knowledge(&self, entry: &KnowledgeEntry) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO knowledge (id, kind, content, created_at, expires_at, last_read_at, is_active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &entry.id[..],
                knowledge_kind_str(entry.kind),
                entry.content,
                entry.created_at,
                entry.expires_at,
                entry.last_read_at,
                entry.is_active as i32,
            ],
        )?;
        Ok(())
    }

    pub fn get_knowledge(&self, id: &Hash32) -> Result<Option<KnowledgeEntry>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, content, created_at, expires_at, last_read_at, is_active
             FROM knowledge WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![&id[..]], row_to_knowledge)?;
        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /// Updates `last_read_at` and `expires_at` for the given knowledge entry.
    pub fn touch_knowledge(
        &self,
        id: &Hash32,
        new_expiry: u64,
        now: u64,
    ) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE knowledge SET last_read_at = ?1, expires_at = ?2 WHERE id = ?3",
            params![now, new_expiry, &id[..]],
        )?;
        Ok(())
    }

    /// Sets `is_active = 0`. Returns `true` if the entry existed.
    pub fn forget_knowledge(&self, id: &Hash32) -> Result<bool, rusqlite::Error> {
        let exists: bool = self
            .conn
            .query_row("SELECT COUNT(*) FROM knowledge WHERE id = ?1", params![&id[..]], |row| {
                row.get::<_, i64>(0)
            })
            .map(|c| c > 0)?;
        if exists {
            self.conn
                .execute("UPDATE knowledge SET is_active = 0 WHERE id = ?1", params![&id[..]])?;
        }
        Ok(exists)
    }

    pub fn get_active_knowledge_count(&self) -> Result<u64, rusqlite::Error> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM knowledge WHERE is_active = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }

    /// Sets `is_active = 0` for the oldest active entry (by `created_at`).
    pub fn archive_oldest_knowledge(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE knowledge SET is_active = 0
             WHERE id = (
                 SELECT id FROM knowledge
                 WHERE is_active = 1
                 ORDER BY created_at ASC LIMIT 1
             )",
            [],
        )?;
        Ok(())
    }

    /// Returns the `limit` most recently created active entries.
    pub fn get_recent_knowledge(&self, limit: u64) -> Result<Vec<KnowledgeEntry>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, content, created_at, expires_at, last_read_at, is_active
             FROM knowledge WHERE is_active = 1
             ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], row_to_knowledge)?;
        rows.collect()
    }
}

// ── Handoff CRUD ──

impl Database {
    /// Serializes `next_actions` and `todos_snapshot` as JSON before storing.
    pub fn save_handoff(&self, record: &HandoffRecord) -> Result<(), rusqlite::Error> {
        let next_actions_json =
            serde_json::to_string(&record.next_actions).unwrap_or_else(|_| "[]".to_string());
        let todos_snapshot_json =
            serde_json::to_string(&record.todos_snapshot).unwrap_or_else(|_| "[]".to_string());
        self.conn.execute(
            "INSERT INTO handoffs (session_id, timestamp, summary, next_actions_json, todos_snapshot_json)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &record.session_id[..],
                record.timestamp,
                &record.summary[..],
                next_actions_json,
                todos_snapshot_json,
            ],
        )?;
        Ok(())
    }

    /// Returns the most recent handoff by timestamp.
    pub fn get_latest_handoff(&self) -> Result<Option<HandoffRecord>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, timestamp, summary, next_actions_json, todos_snapshot_json
             FROM handoffs ORDER BY timestamp DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map([], |row| {
            let blob: Vec<u8> = row.get(0)?;
            let mut session_id = [0u8; 32];
            let len = blob.len().min(32);
            session_id[..len].copy_from_slice(&blob[..len]);
            let next_actions_json: String = row.get(3)?;
            let todos_json: String = row.get(4)?;
            Ok(HandoffRecord {
                session_id,
                timestamp: row.get(1)?,
                summary: row.get(2)?,
                next_actions: serde_json::from_str(&next_actions_json).unwrap_or_default(),
                todos_snapshot: serde_json::from_str(&todos_json).unwrap_or_default(),
            })
        })?;
        match rows.next() {
            Some(Ok(record)) => Ok(Some(record)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }
}

// ── Message Log ──

impl Database {
    pub fn insert_message(&self, role: &str, content: &str) -> Result<(), rusqlite::Error> {
        let now = now_secs();
        self.conn.execute(
            "INSERT INTO messages (role, content, timestamp) VALUES (?1, ?2, ?3)",
            params![role, content, now],
        )?;
        Ok(())
    }

    /// Returns `(role, content, timestamp)` tuples for the most recent messages.
    pub fn get_recent_messages(
        &self,
        limit: u64,
    ) -> Result<Vec<(String, String, u64)>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT role, content, timestamp FROM messages ORDER BY id DESC LIMIT ?1")?;
        let rows =
            stmt.query_map(params![limit], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        rows.collect()
    }
}

// ── Failure Log ──

impl Database {
    pub fn record_failure(&self, record: &FailureRecord) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO failures (timestamp, tool_name, input_hash, error_kind)
             VALUES (?1, ?2, ?3, ?4)",
            params![record.timestamp, record.tool_name, &record.input_hash[..], record.error_kind,],
        )?;
        Ok(())
    }

    /// Returns failure records for `tool_name` with `timestamp >= since`.
    pub fn recent_failures(
        &self,
        tool_name: &str,
        since: u64,
    ) -> Result<Vec<FailureRecord>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, tool_name, input_hash, error_kind
             FROM failures
             WHERE tool_name = ?1 AND timestamp >= ?2
             ORDER BY timestamp DESC",
        )?;
        let rows = stmt.query_map(params![tool_name, since], |row| {
            let blob: Vec<u8> = row.get(2)?;
            let mut input_hash = [0u8; 32];
            let len = blob.len().min(32);
            input_hash[..len].copy_from_slice(&blob[..len]);
            Ok(FailureRecord {
                timestamp: row.get(0)?,
                tool_name: row.get(1)?,
                input_hash,
                error_kind: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    /// Counts all failures with `timestamp >= since`.
    pub fn count_failures_since(&self, since: u64) -> Result<u64, rusqlite::Error> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM failures WHERE timestamp >= ?1",
            params![since],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }
}

// ── State KV ──

impl Database {
    /// Inserts or replaces a key-value pair in the `state` table.
    pub fn set_state(&self, key: &str, value: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO state (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_state(&self, key: &str) -> Result<Option<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT value FROM state WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get(0))?;
        match rows.next() {
            Some(Ok(val)) => Ok(Some(val)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }
}

// ── Helpers ──

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn todo_status_str(s: TodoStatus) -> &'static str {
    match s {
        TodoStatus::Pending => "Pending",
        TodoStatus::InProgress => "InProgress",
        TodoStatus::Blocked => "Blocked",
        TodoStatus::Done => "Done",
        TodoStatus::Cancelled => "Cancelled",
    }
}

fn parse_todo_status(s: &str) -> TodoStatus {
    match s {
        "Pending" => TodoStatus::Pending,
        "InProgress" => TodoStatus::InProgress,
        "Blocked" => TodoStatus::Blocked,
        "Done" => TodoStatus::Done,
        "Cancelled" => TodoStatus::Cancelled,
        _ => TodoStatus::Pending,
    }
}

fn knowledge_kind_str(k: KnowledgeKind) -> &'static str {
    match k {
        KnowledgeKind::Finding => "Finding",
        KnowledgeKind::Pattern => "Pattern",
        KnowledgeKind::Constraint => "Constraint",
        KnowledgeKind::Decision => "Decision",
    }
}

fn parse_knowledge_kind(s: &str) -> KnowledgeKind {
    match s {
        "Finding" => KnowledgeKind::Finding,
        "Pattern" => KnowledgeKind::Pattern,
        "Constraint" => KnowledgeKind::Constraint,
        "Decision" => KnowledgeKind::Decision,
        _ => KnowledgeKind::Finding,
    }
}

fn row_to_knowledge(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeEntry> {
    let blob: Vec<u8> = row.get(0)?;
    let mut id = [0u8; 32];
    let len = blob.len().min(32);
    id[..len].copy_from_slice(&blob[..len]);
    Ok(KnowledgeEntry {
        id,
        kind: parse_knowledge_kind(&row.get::<_, String>(1)?),
        content: row.get(2)?,
        created_at: row.get(3)?,
        expires_at: row.get(4)?,
        last_read_at: row.get(5)?,
        is_active: row.get::<_, i32>(6)? != 0,
    })
}
