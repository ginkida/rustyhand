//! MemorySubstrate: unified implementation of the `Memory` trait.
//!
//! Composes the structured store, semantic store, knowledge store,
//! session store, and consolidation engine behind a single async API.

use crate::consolidation::ConsolidationEngine;
use crate::knowledge::KnowledgeStore;
use crate::migration::run_migrations;
use crate::semantic::SemanticStore;
use crate::session::{Session, SessionStore};
use crate::structured::StructuredStore;
use crate::usage::UsageStore;

use async_trait::async_trait;
use rusqlite::types::{Value, ValueRef};
use rusqlite::Connection;
use rusty_hand_types::agent::{AgentEntry, AgentId, SessionId};
use rusty_hand_types::error::{RustyHandError, RustyHandResult};
use rusty_hand_types::memory::{
    ConsolidationReport, Entity, ExportFormat, GraphMatch, GraphPattern, ImportReport, Memory,
    MemoryFilter, MemoryFragment, MemoryId, MemorySource, Relation,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// The unified memory substrate. Implements the `Memory` trait by delegating
/// to specialized stores backed by a shared SQLite connection.
pub struct MemorySubstrate {
    conn: Arc<Mutex<Connection>>,
    structured: StructuredStore,
    semantic: SemanticStore,
    knowledge: KnowledgeStore,
    sessions: SessionStore,
    consolidation: ConsolidationEngine,
    usage: UsageStore,
}

#[derive(Debug, Serialize, Deserialize)]
struct DatabaseSnapshot {
    schema_version: u32,
    exported_at: String,
    tables: Vec<TableSnapshot>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TableSnapshot {
    name: String,
    columns: Vec<String>,
    rows: Vec<Vec<SnapshotValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
enum SnapshotValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl SnapshotValue {
    fn from_value_ref(value: ValueRef<'_>) -> Self {
        match value {
            ValueRef::Null => Self::Null,
            ValueRef::Integer(v) => Self::Integer(v),
            ValueRef::Real(v) => Self::Real(v),
            ValueRef::Text(v) => Self::Text(String::from_utf8_lossy(v).into_owned()),
            ValueRef::Blob(v) => Self::Blob(v.to_vec()),
        }
    }

    fn into_sql_value(self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Integer(v) => Value::Integer(v),
            Self::Real(v) => Value::Real(v),
            Self::Text(v) => Value::Text(v),
            Self::Blob(v) => Value::Blob(v),
        }
    }
}

impl MemorySubstrate {
    /// Open or create a memory substrate at the given database path.
    pub fn open(db_path: &Path, decay_rate: f32) -> RustyHandResult<Self> {
        let conn = Connection::open(db_path).map_err(|e| RustyHandError::Memory(e.to_string()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        run_migrations(&conn).map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let shared = Arc::new(Mutex::new(conn));

        Ok(Self {
            conn: Arc::clone(&shared),
            structured: StructuredStore::new(Arc::clone(&shared)),
            semantic: SemanticStore::new(Arc::clone(&shared)),
            knowledge: KnowledgeStore::new(Arc::clone(&shared)),
            sessions: SessionStore::new(Arc::clone(&shared)),
            usage: UsageStore::new(Arc::clone(&shared)),
            consolidation: ConsolidationEngine::new(shared, decay_rate),
        })
    }

    /// Create an in-memory substrate (for testing).
    pub fn open_in_memory(decay_rate: f32) -> RustyHandResult<Self> {
        let conn =
            Connection::open_in_memory().map_err(|e| RustyHandError::Memory(e.to_string()))?;
        run_migrations(&conn).map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let shared = Arc::new(Mutex::new(conn));

        Ok(Self {
            conn: Arc::clone(&shared),
            structured: StructuredStore::new(Arc::clone(&shared)),
            semantic: SemanticStore::new(Arc::clone(&shared)),
            knowledge: KnowledgeStore::new(Arc::clone(&shared)),
            sessions: SessionStore::new(Arc::clone(&shared)),
            usage: UsageStore::new(Arc::clone(&shared)),
            consolidation: ConsolidationEngine::new(shared, decay_rate),
        })
    }

    /// Get a reference to the usage store.
    pub fn usage(&self) -> &UsageStore {
        &self.usage
    }

    /// Get the shared database connection (for constructing stores from outside).
    pub fn usage_conn(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    /// Save an agent entry to persistent storage.
    pub fn save_agent(&self, entry: &AgentEntry) -> RustyHandResult<()> {
        self.structured.save_agent(entry)
    }

    /// Load an agent entry from persistent storage.
    pub fn load_agent(&self, agent_id: AgentId) -> RustyHandResult<Option<AgentEntry>> {
        self.structured.load_agent(agent_id)
    }

    /// Remove an agent from persistent storage and cascade-delete sessions.
    pub fn remove_agent(&self, agent_id: AgentId) -> RustyHandResult<()> {
        // Delete associated sessions first
        let _ = self.sessions.delete_agent_sessions(agent_id);
        self.structured.remove_agent(agent_id)
    }

    /// Load all agent entries from persistent storage.
    pub fn load_all_agents(&self) -> RustyHandResult<Vec<AgentEntry>> {
        self.structured.load_all_agents()
    }

    /// List all saved agents.
    pub fn list_agents(&self) -> RustyHandResult<Vec<(String, String, String)>> {
        self.structured.list_agents()
    }

    /// Synchronous get from the structured store (for kernel handle use).
    pub fn structured_get(
        &self,
        agent_id: AgentId,
        key: &str,
    ) -> RustyHandResult<Option<serde_json::Value>> {
        self.structured.get(agent_id, key)
    }

    /// List all KV pairs for an agent.
    pub fn list_kv(&self, agent_id: AgentId) -> RustyHandResult<Vec<(String, serde_json::Value)>> {
        self.structured.list_kv(agent_id)
    }

    /// Delete a KV entry for an agent.
    pub fn structured_delete(&self, agent_id: AgentId, key: &str) -> RustyHandResult<()> {
        self.structured.delete(agent_id, key)
    }

    /// Synchronous set in the structured store (for kernel handle use).
    pub fn structured_set(
        &self,
        agent_id: AgentId,
        key: &str,
        value: serde_json::Value,
    ) -> RustyHandResult<()> {
        self.structured.set(agent_id, key, value)
    }

    /// Get a session by ID.
    pub fn get_session(&self, session_id: SessionId) -> RustyHandResult<Option<Session>> {
        self.sessions.get_session(session_id)
    }

    /// Save a session.
    pub fn save_session(&self, session: &Session) -> RustyHandResult<()> {
        self.sessions.save_session(session)
    }

    /// Create a new empty session for an agent.
    pub fn create_session(&self, agent_id: AgentId) -> RustyHandResult<Session> {
        self.sessions.create_session(agent_id)
    }

    /// List all sessions with metadata.
    pub fn list_sessions(&self) -> RustyHandResult<Vec<serde_json::Value>> {
        self.sessions.list_sessions()
    }

    /// Delete a session by ID.
    pub fn delete_session(&self, session_id: SessionId) -> RustyHandResult<()> {
        self.sessions.delete_session(session_id)
    }

    /// Set or clear a session label.
    pub fn set_session_label(
        &self,
        session_id: SessionId,
        label: Option<&str>,
    ) -> RustyHandResult<()> {
        self.sessions.set_session_label(session_id, label)
    }

    /// Find a session by label for a given agent.
    pub fn find_session_by_label(
        &self,
        agent_id: AgentId,
        label: &str,
    ) -> RustyHandResult<Option<Session>> {
        self.sessions.find_session_by_label(agent_id, label)
    }

    /// List all sessions for a specific agent.
    pub fn list_agent_sessions(
        &self,
        agent_id: AgentId,
    ) -> RustyHandResult<Vec<serde_json::Value>> {
        self.sessions.list_agent_sessions(agent_id)
    }

    /// Create a new session with an optional label.
    pub fn create_session_with_label(
        &self,
        agent_id: AgentId,
        label: Option<&str>,
    ) -> RustyHandResult<Session> {
        self.sessions.create_session_with_label(agent_id, label)
    }

    /// Load canonical session context for cross-channel memory.
    ///
    /// Returns the compacted summary (if any) and recent messages from the
    /// agent's persistent canonical session.
    pub fn canonical_context(
        &self,
        agent_id: AgentId,
        window_size: Option<usize>,
    ) -> RustyHandResult<(Option<String>, Vec<rusty_hand_types::message::Message>)> {
        self.sessions.canonical_context(agent_id, window_size)
    }

    /// Store an LLM-generated summary, replacing older messages with the kept subset.
    ///
    /// Used by the compactor to replace text-truncation compaction with an
    /// LLM-generated summary of older conversation history.
    pub fn store_llm_summary(
        &self,
        agent_id: AgentId,
        summary: &str,
        kept_messages: Vec<rusty_hand_types::message::Message>,
    ) -> RustyHandResult<()> {
        self.sessions
            .store_llm_summary(agent_id, summary, kept_messages)
    }

    /// Write a human-readable JSONL mirror of a session to disk.
    ///
    /// Best-effort — errors are returned but should be logged,
    /// never affecting the primary SQLite store.
    pub fn write_jsonl_mirror(
        &self,
        session: &Session,
        sessions_dir: &Path,
    ) -> Result<(), std::io::Error> {
        self.sessions.write_jsonl_mirror(session, sessions_dir)
    }

    /// Append messages to the agent's canonical session for cross-channel persistence.
    pub fn append_canonical(
        &self,
        agent_id: AgentId,
        messages: &[rusty_hand_types::message::Message],
        compaction_threshold: Option<usize>,
    ) -> RustyHandResult<()> {
        self.sessions
            .append_canonical(agent_id, messages, compaction_threshold)?;
        Ok(())
    }

    // -----------------------------------------------------------------
    // Paired devices persistence
    // -----------------------------------------------------------------

    /// Load all paired devices from the database.
    pub fn load_paired_devices(&self) -> RustyHandResult<Vec<serde_json::Value>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let mut stmt = conn.prepare(
            "SELECT device_id, display_name, platform, paired_at, last_seen, push_token FROM paired_devices"
        ).map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "device_id": row.get::<_, String>(0)?,
                    "display_name": row.get::<_, String>(1)?,
                    "platform": row.get::<_, String>(2)?,
                    "paired_at": row.get::<_, String>(3)?,
                    "last_seen": row.get::<_, String>(4)?,
                    "push_token": row.get::<_, Option<String>>(5)?,
                }))
            })
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let mut devices = Vec::new();
        for row in rows {
            devices.push(row.map_err(|e| RustyHandError::Memory(e.to_string()))?);
        }
        Ok(devices)
    }

    /// Save a paired device to the database (insert or replace).
    pub fn save_paired_device(
        &self,
        device_id: &str,
        display_name: &str,
        platform: &str,
        paired_at: &str,
        last_seen: &str,
        push_token: Option<&str>,
    ) -> RustyHandResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO paired_devices (device_id, display_name, platform, paired_at, last_seen, push_token) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![device_id, display_name, platform, paired_at, last_seen, push_token],
        ).map_err(|e| RustyHandError::Memory(e.to_string()))?;
        Ok(())
    }

    /// Remove a paired device from the database.
    pub fn remove_paired_device(&self, device_id: &str) -> RustyHandResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        conn.execute(
            "DELETE FROM paired_devices WHERE device_id = ?1",
            rusqlite::params![device_id],
        )
        .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        Ok(())
    }

    // -----------------------------------------------------------------
    // Embedding-aware memory operations
    // -----------------------------------------------------------------

    /// Store a memory with an embedding vector.
    pub fn remember_with_embedding(
        &self,
        agent_id: AgentId,
        content: &str,
        source: MemorySource,
        scope: &str,
        metadata: HashMap<String, serde_json::Value>,
        embedding: Option<&[f32]>,
    ) -> RustyHandResult<MemoryId> {
        self.semantic
            .remember_with_embedding(agent_id, content, source, scope, metadata, embedding)
    }

    /// Recall memories using vector similarity when a query embedding is provided.
    pub fn recall_with_embedding(
        &self,
        query: &str,
        limit: usize,
        filter: Option<MemoryFilter>,
        query_embedding: Option<&[f32]>,
    ) -> RustyHandResult<Vec<MemoryFragment>> {
        self.semantic
            .recall_with_embedding(query, limit, filter, query_embedding)
    }

    /// Update the embedding for an existing memory.
    pub fn update_embedding(&self, id: MemoryId, embedding: &[f32]) -> RustyHandResult<()> {
        self.semantic.update_embedding(id, embedding)
    }

    /// Async wrapper for `recall_with_embedding` — runs in a blocking thread.
    pub async fn recall_with_embedding_async(
        &self,
        query: &str,
        limit: usize,
        filter: Option<MemoryFilter>,
        query_embedding: Option<&[f32]>,
    ) -> RustyHandResult<Vec<MemoryFragment>> {
        let store = self.semantic.clone();
        let query = query.to_string();
        let embedding_owned = query_embedding.map(|e| e.to_vec());
        tokio::task::spawn_blocking(move || {
            store.recall_with_embedding(&query, limit, filter, embedding_owned.as_deref())
        })
        .await
        .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    /// Async wrapper for `remember_with_embedding` — runs in a blocking thread.
    pub async fn remember_with_embedding_async(
        &self,
        agent_id: AgentId,
        content: &str,
        source: MemorySource,
        scope: &str,
        metadata: HashMap<String, serde_json::Value>,
        embedding: Option<&[f32]>,
    ) -> RustyHandResult<MemoryId> {
        let store = self.semantic.clone();
        let content = content.to_string();
        let scope = scope.to_string();
        let embedding_owned = embedding.map(|e| e.to_vec());
        tokio::task::spawn_blocking(move || {
            store.remember_with_embedding(
                agent_id,
                &content,
                source,
                &scope,
                metadata,
                embedding_owned.as_deref(),
            )
        })
        .await
        .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    // -----------------------------------------------------------------
    // Task queue operations
    // -----------------------------------------------------------------

    /// Post a new task to the shared queue. Returns the task ID.
    pub async fn task_post(
        &self,
        title: &str,
        description: &str,
        assigned_to: Option<&str>,
        created_by: Option<&str>,
    ) -> RustyHandResult<String> {
        let conn = Arc::clone(&self.conn);
        let title = title.to_string();
        let description = description.to_string();
        let assigned_to = assigned_to.unwrap_or("").to_string();
        let created_by = created_by.unwrap_or("").to_string();

        tokio::task::spawn_blocking(move || {
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            let db = conn.lock().map_err(|e| RustyHandError::Internal(e.to_string()))?;
            db.execute(
                "INSERT INTO task_queue (id, agent_id, task_type, payload, status, priority, created_at, title, description, assigned_to, created_by)
                 VALUES (?1, ?2, ?3, ?4, 'pending', 0, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![id, &created_by, &title, b"", now, title, description, assigned_to, created_by],
            )
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
            Ok(id)
        })
        .await
        .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    /// Claim the next pending task (optionally for a specific assignee). Returns task JSON or None.
    pub async fn task_claim(&self, agent_id: &str) -> RustyHandResult<Option<serde_json::Value>> {
        let conn = Arc::clone(&self.conn);
        let agent_id = agent_id.to_string();

        tokio::task::spawn_blocking(move || {
            let db = conn.lock().map_err(|e| RustyHandError::Internal(e.to_string()))?;
            // Find first pending task assigned to this agent, or any unassigned pending task
            let mut stmt = db.prepare(
                "SELECT id, title, description, assigned_to, created_by, created_at
                 FROM task_queue
                 WHERE status = 'pending' AND (assigned_to = ?1 OR assigned_to = '')
                 ORDER BY priority DESC, created_at ASC
                 LIMIT 1"
            ).map_err(|e| RustyHandError::Memory(e.to_string()))?;

            let result = stmt.query_row(rusqlite::params![agent_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            });

            match result {
                Ok((id, title, description, assigned, created_by, created_at)) => {
                    // Update status to in_progress
                    db.execute(
                        "UPDATE task_queue SET status = 'in_progress', assigned_to = ?2 WHERE id = ?1",
                        rusqlite::params![id, agent_id],
                    ).map_err(|e| RustyHandError::Memory(e.to_string()))?;

                    Ok(Some(serde_json::json!({
                        "id": id,
                        "title": title,
                        "description": description,
                        "status": "in_progress",
                        "assigned_to": if assigned.is_empty() { &agent_id } else { &assigned },
                        "created_by": created_by,
                        "created_at": created_at,
                    })))
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(RustyHandError::Memory(e.to_string())),
            }
        })
        .await
        .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    /// Mark a task as completed with a result string.
    pub async fn task_complete(&self, task_id: &str, result: &str) -> RustyHandResult<()> {
        let conn = Arc::clone(&self.conn);
        let task_id = task_id.to_string();
        let result = result.to_string();

        tokio::task::spawn_blocking(move || {
            let now = chrono::Utc::now().to_rfc3339();
            let db = conn.lock().map_err(|e| RustyHandError::Internal(e.to_string()))?;
            let rows = db.execute(
                "UPDATE task_queue SET status = 'completed', result = ?2, completed_at = ?3 WHERE id = ?1",
                rusqlite::params![task_id, result, now],
            ).map_err(|e| RustyHandError::Memory(e.to_string()))?;
            if rows == 0 {
                return Err(RustyHandError::Internal(format!("Task not found: {task_id}")));
            }
            Ok(())
        })
        .await
        .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    /// List tasks, optionally filtered by status.
    pub async fn task_list(&self, status: Option<&str>) -> RustyHandResult<Vec<serde_json::Value>> {
        let conn = Arc::clone(&self.conn);
        let status = status.map(|s| s.to_string());

        tokio::task::spawn_blocking(move || {
            let db = conn.lock().map_err(|e| RustyHandError::Internal(e.to_string()))?;
            let (sql, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match &status {
                Some(s) => (
                    "SELECT id, title, description, status, assigned_to, created_by, created_at, completed_at, result FROM task_queue WHERE status = ?1 ORDER BY created_at DESC",
                    vec![Box::new(s.clone())],
                ),
                None => (
                    "SELECT id, title, description, status, assigned_to, created_by, created_at, completed_at, result FROM task_queue ORDER BY created_at DESC",
                    vec![],
                ),
            };

            let mut stmt = db.prepare(sql).map_err(|e| RustyHandError::Memory(e.to_string()))?;
            let params_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            let rows = stmt.query_map(params_refs.as_slice(), |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "title": row.get::<_, String>(1).unwrap_or_default(),
                    "description": row.get::<_, String>(2).unwrap_or_default(),
                    "status": row.get::<_, String>(3)?,
                    "assigned_to": row.get::<_, String>(4).unwrap_or_default(),
                    "created_by": row.get::<_, String>(5).unwrap_or_default(),
                    "created_at": row.get::<_, String>(6).unwrap_or_default(),
                    "completed_at": row.get::<_, Option<String>>(7).unwrap_or(None),
                    "result": row.get::<_, Option<String>>(8).unwrap_or(None),
                }))
            }).map_err(|e| RustyHandError::Memory(e.to_string()))?;

            let mut tasks = Vec::new();
            for row in rows {
                tasks.push(row.map_err(|e| RustyHandError::Memory(e.to_string()))?);
            }
            Ok(tasks)
        })
        .await
        .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }
}

fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn list_user_tables(conn: &Connection) -> RustyHandResult<Vec<String>> {
    let mut stmt = conn
        .prepare(
            "SELECT name
             FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .map_err(|e| RustyHandError::Memory(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| RustyHandError::Memory(e.to_string()))?;

    let mut tables = Vec::new();
    for row in rows {
        tables.push(row.map_err(|e| RustyHandError::Memory(e.to_string()))?);
    }
    Ok(tables)
}

fn table_columns(conn: &Connection, table: &str) -> RustyHandResult<Vec<String>> {
    let sql = format!("PRAGMA table_info({})", quote_identifier(table));
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| RustyHandError::Memory(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| RustyHandError::Memory(e.to_string()))?;

    let mut columns = Vec::new();
    for row in rows {
        columns.push(row.map_err(|e| RustyHandError::Memory(e.to_string()))?);
    }
    Ok(columns)
}

fn export_snapshot(conn: &Connection) -> RustyHandResult<DatabaseSnapshot> {
    let schema_version = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| RustyHandError::Memory(e.to_string()))?;
    let mut tables = Vec::new();

    for table in list_user_tables(conn)? {
        let columns = table_columns(conn, &table)?;
        let select_columns = columns
            .iter()
            .map(|column| quote_identifier(column))
            .collect::<Vec<_>>()
            .join(", ");
        // SECURITY: exclude soft-deleted memories from export to prevent data leakage
        let sql = if table == "memories" {
            format!(
                "SELECT {select_columns} FROM {} WHERE deleted = 0",
                quote_identifier(&table)
            )
        } else {
            format!("SELECT {select_columns} FROM {}", quote_identifier(&table))
        };
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let mut values = Vec::with_capacity(columns.len());
                for idx in 0..columns.len() {
                    values.push(SnapshotValue::from_value_ref(row.get_ref(idx)?));
                }
                Ok(values)
            })
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;

        let mut snapshot_rows = Vec::new();
        for row in rows {
            snapshot_rows.push(row.map_err(|e| RustyHandError::Memory(e.to_string()))?);
        }

        tables.push(TableSnapshot {
            name: table,
            columns,
            rows: snapshot_rows,
        });
    }

    Ok(DatabaseSnapshot {
        schema_version,
        exported_at: chrono::Utc::now().to_rfc3339(),
        tables,
    })
}

fn encode_snapshot(snapshot: &DatabaseSnapshot, format: ExportFormat) -> RustyHandResult<Vec<u8>> {
    match format {
        ExportFormat::Json => serde_json::to_vec_pretty(snapshot)
            .map_err(|e| RustyHandError::Serialization(e.to_string())),
        ExportFormat::MessagePack => rmp_serde::to_vec_named(snapshot)
            .map_err(|e| RustyHandError::Serialization(e.to_string())),
    }
}

fn decode_snapshot(data: &[u8], format: ExportFormat) -> RustyHandResult<DatabaseSnapshot> {
    match format {
        ExportFormat::Json => {
            serde_json::from_slice(data).map_err(|e| RustyHandError::Serialization(e.to_string()))
        }
        ExportFormat::MessagePack => {
            rmp_serde::from_slice(data).map_err(|e| RustyHandError::Serialization(e.to_string()))
        }
    }
}

fn table_row_count(conn: &Connection, table: &str) -> RustyHandResult<u64> {
    let sql = format!("SELECT COUNT(*) FROM {}", quote_identifier(table));
    conn.query_row(&sql, [], |row| row.get::<_, i64>(0))
        .map(|count| count as u64)
        .map_err(|e| RustyHandError::Memory(e.to_string()))
}

#[async_trait]
impl Memory for MemorySubstrate {
    async fn get(
        &self,
        agent_id: AgentId,
        key: &str,
    ) -> RustyHandResult<Option<serde_json::Value>> {
        let store = self.structured.clone();
        let key = key.to_string();
        tokio::task::spawn_blocking(move || store.get(agent_id, &key))
            .await
            .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    async fn set(
        &self,
        agent_id: AgentId,
        key: &str,
        value: serde_json::Value,
    ) -> RustyHandResult<()> {
        let store = self.structured.clone();
        let key = key.to_string();
        tokio::task::spawn_blocking(move || store.set(agent_id, &key, value))
            .await
            .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    async fn delete(&self, agent_id: AgentId, key: &str) -> RustyHandResult<()> {
        let store = self.structured.clone();
        let key = key.to_string();
        tokio::task::spawn_blocking(move || store.delete(agent_id, &key))
            .await
            .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    async fn remember(
        &self,
        agent_id: AgentId,
        content: &str,
        source: MemorySource,
        scope: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> RustyHandResult<MemoryId> {
        let store = self.semantic.clone();
        let content = content.to_string();
        let scope = scope.to_string();
        tokio::task::spawn_blocking(move || {
            store.remember(agent_id, &content, source, &scope, metadata)
        })
        .await
        .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        filter: Option<MemoryFilter>,
    ) -> RustyHandResult<Vec<MemoryFragment>> {
        let store = self.semantic.clone();
        let query = query.to_string();
        tokio::task::spawn_blocking(move || store.recall(&query, limit, filter))
            .await
            .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    async fn forget(&self, id: MemoryId) -> RustyHandResult<()> {
        let store = self.semantic.clone();
        tokio::task::spawn_blocking(move || store.forget(id))
            .await
            .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    async fn add_entity(&self, entity: Entity) -> RustyHandResult<String> {
        let store = self.knowledge.clone();
        tokio::task::spawn_blocking(move || store.add_entity(entity))
            .await
            .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    async fn add_relation(&self, relation: Relation) -> RustyHandResult<String> {
        let store = self.knowledge.clone();
        tokio::task::spawn_blocking(move || store.add_relation(relation))
            .await
            .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    async fn query_graph(&self, pattern: GraphPattern) -> RustyHandResult<Vec<GraphMatch>> {
        let store = self.knowledge.clone();
        tokio::task::spawn_blocking(move || store.query_graph(pattern))
            .await
            .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    async fn consolidate(&self) -> RustyHandResult<ConsolidationReport> {
        let engine = self.consolidation.clone();
        tokio::task::spawn_blocking(move || engine.consolidate())
            .await
            .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    async fn export(&self, format: ExportFormat) -> RustyHandResult<Vec<u8>> {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let db = conn
                .lock()
                .map_err(|e| RustyHandError::Internal(e.to_string()))?;
            let snapshot = export_snapshot(&db)?;
            encode_snapshot(&snapshot, format)
        })
        .await
        .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }

    async fn import(&self, data: &[u8], format: ExportFormat) -> RustyHandResult<ImportReport> {
        let snapshot = decode_snapshot(data, format)?;
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let mut db = conn
                .lock()
                .map_err(|e| RustyHandError::Internal(e.to_string()))?;
            let tx = db
                .transaction()
                .map_err(|e| RustyHandError::Memory(e.to_string()))?;

            for table in list_user_tables(&tx)? {
                let sql = format!("DELETE FROM {}", quote_identifier(&table));
                tx.execute(&sql, [])
                    .map_err(|e| RustyHandError::Memory(e.to_string()))?;
            }

            for table in snapshot.tables {
                if table.columns.is_empty() {
                    continue;
                }

                let placeholders = (1..=table.columns.len())
                    .map(|idx| format!("?{idx}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                let columns = table
                    .columns
                    .iter()
                    .map(|column| quote_identifier(column))
                    .collect::<Vec<_>>()
                    .join(", ");
                let sql = format!(
                    "INSERT INTO {} ({columns}) VALUES ({placeholders})",
                    quote_identifier(&table.name),
                );
                let mut stmt = tx
                    .prepare(&sql)
                    .map_err(|e| RustyHandError::Memory(e.to_string()))?;

                for row in table.rows {
                    if row.len() != table.columns.len() {
                        return Err(RustyHandError::Serialization(format!(
                            "Invalid backup row for table {}: expected {} values, found {}",
                            table.name,
                            table.columns.len(),
                            row.len()
                        )));
                    }
                    let values = row
                        .into_iter()
                        .map(SnapshotValue::into_sql_value)
                        .collect::<Vec<_>>();
                    stmt.execute(rusqlite::params_from_iter(values.iter()))
                        .map_err(|e| RustyHandError::Memory(e.to_string()))?;
                }
            }

            tx.pragma_update(None, "user_version", snapshot.schema_version)
                .map_err(|e| RustyHandError::Memory(e.to_string()))?;

            let report = ImportReport {
                entities_imported: table_row_count(&tx, "entities")?,
                relations_imported: table_row_count(&tx, "relations")?,
                memories_imported: table_row_count(&tx, "memories")?,
                errors: Vec::new(),
            };

            tx.commit()
                .map_err(|e| RustyHandError::Memory(e.to_string()))?;

            Ok(report)
        })
        .await
        .map_err(|e| RustyHandError::Internal(e.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rusty_hand_types::memory::{EntityType, RelationType};

    #[tokio::test]
    async fn test_substrate_kv() {
        let substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
        let agent_id = AgentId::new();
        substrate
            .set(agent_id, "key", serde_json::json!("value"))
            .await
            .unwrap();
        let val = substrate.get(agent_id, "key").await.unwrap();
        assert_eq!(val, Some(serde_json::json!("value")));
    }

    #[tokio::test]
    async fn test_substrate_remember_recall() {
        let substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
        let agent_id = AgentId::new();
        substrate
            .remember(
                agent_id,
                "Rust is a great language",
                MemorySource::Conversation,
                "episodic",
                HashMap::new(),
            )
            .await
            .unwrap();
        let results = substrate.recall("Rust", 10, None).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_task_post_and_list() {
        let substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
        let id = substrate
            .task_post(
                "Review code",
                "Check the auth module for issues",
                Some("auditor"),
                Some("orchestrator"),
            )
            .await
            .unwrap();
        assert!(!id.is_empty());

        let tasks = substrate.task_list(Some("pending")).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["title"], "Review code");
        assert_eq!(tasks[0]["assigned_to"], "auditor");
        assert_eq!(tasks[0]["status"], "pending");
    }

    #[tokio::test]
    async fn test_task_claim_and_complete() {
        let substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
        let task_id = substrate
            .task_post(
                "Audit endpoint",
                "Security audit the /api/login endpoint",
                Some("auditor"),
                None,
            )
            .await
            .unwrap();

        // Claim the task
        let claimed = substrate.task_claim("auditor").await.unwrap();
        assert!(claimed.is_some());
        let claimed = claimed.unwrap();
        assert_eq!(claimed["id"], task_id);
        assert_eq!(claimed["status"], "in_progress");

        // Complete the task
        substrate
            .task_complete(&task_id, "No vulnerabilities found")
            .await
            .unwrap();

        // Verify it shows as completed
        let tasks = substrate.task_list(Some("completed")).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0]["result"], "No vulnerabilities found");
    }

    #[tokio::test]
    async fn test_task_claim_empty() {
        let substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
        let claimed = substrate.task_claim("nobody").await.unwrap();
        assert!(claimed.is_none());
    }

    #[tokio::test]
    async fn test_export_import_roundtrip_json() {
        let source = MemorySubstrate::open_in_memory(0.1).unwrap();
        let agent_id = AgentId::new();
        source
            .set(agent_id, "project", serde_json::json!("rustyhand"))
            .await
            .unwrap();
        source
            .remember(
                agent_id,
                "Backup and restore must be lossless",
                MemorySource::Conversation,
                "episodic",
                HashMap::new(),
            )
            .await
            .unwrap();
        let entity_id = source
            .add_entity(Entity {
                id: "project-rustyhand".to_string(),
                entity_type: EntityType::Project,
                name: "RustyHand".to_string(),
                properties: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .await
            .unwrap();
        source
            .add_relation(Relation {
                source: entity_id.clone(),
                relation: RelationType::DependsOn,
                target: entity_id,
                properties: HashMap::new(),
                confidence: 1.0,
                created_at: Utc::now(),
            })
            .await
            .unwrap();

        let backup = source.export(ExportFormat::Json).await.unwrap();

        let restored = MemorySubstrate::open_in_memory(0.1).unwrap();
        let report = restored.import(&backup, ExportFormat::Json).await.unwrap();

        assert_eq!(report.memories_imported, 1);
        assert_eq!(report.entities_imported, 1);
        assert_eq!(report.relations_imported, 1);
        assert_eq!(
            restored.get(agent_id, "project").await.unwrap(),
            Some(serde_json::json!("rustyhand"))
        );
        assert_eq!(
            restored.recall("lossless", 10, None).await.unwrap().len(),
            1
        );
    }

    #[tokio::test]
    async fn test_export_import_roundtrip_messagepack() {
        let source = MemorySubstrate::open_in_memory(0.1).unwrap();
        let agent_id = AgentId::new();
        source
            .set(agent_id, "format", serde_json::json!("messagepack"))
            .await
            .unwrap();

        let backup = source.export(ExportFormat::MessagePack).await.unwrap();

        let restored = MemorySubstrate::open_in_memory(0.1).unwrap();
        restored
            .import(&backup, ExportFormat::MessagePack)
            .await
            .unwrap();

        assert_eq!(
            restored.get(agent_id, "format").await.unwrap(),
            Some(serde_json::json!("messagepack"))
        );
    }
}
