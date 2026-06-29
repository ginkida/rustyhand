//! Memory consolidation and decay logic.
//!
//! Reduces confidence of old, unaccessed memories and merges
//! duplicate/similar memories.

use chrono::Utc;
use rusqlite::Connection;
use rusty_hand_types::error::{RustyHandError, RustyHandResult};
use rusty_hand_types::memory::ConsolidationReport;
use std::sync::{Arc, Mutex};

/// Memory consolidation engine.
#[derive(Clone)]
pub struct ConsolidationEngine {
    conn: Arc<Mutex<Connection>>,
    /// Decay rate: how much to reduce confidence per consolidation cycle.
    decay_rate: f32,
}

impl ConsolidationEngine {
    /// Create a new consolidation engine.
    pub fn new(conn: Arc<Mutex<Connection>>, decay_rate: f32) -> Self {
        Self { conn, decay_rate }
    }

    /// Run a consolidation cycle: decay old memories.
    pub fn consolidate(&self) -> RustyHandResult<ConsolidationReport> {
        let start = std::time::Instant::now();
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Internal(e.to_string()))?;

        // Wrap the decay + merge in a single transaction so a mid-operation
        // failure rolls back cleanly. Without this, an aborted merge can leave
        // the survivor having absorbed the group's total access_count while some
        // losers stay live, so the next cycle re-merges and double-counts.
        // We only hold `&Connection` (via the MutexGuard), so use
        // `unchecked_transaction()` rather than `transaction()` (which needs
        // `&mut self`). Dropping `tx` on an early return rolls back.
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;

        // Decay confidence of memories not accessed in the last 7 days
        let cutoff = (Utc::now() - chrono::Duration::days(7)).to_rfc3339();
        let decay_factor = 1.0 - self.decay_rate as f64;

        let decayed = conn
            .execute(
                "UPDATE memories SET confidence = MAX(0.1, confidence * ?1)
                 WHERE deleted = 0 AND accessed_at < ?2 AND confidence > 0.1",
                rusqlite::params![decay_factor, cutoff],
            )
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;

        // Merge exact-duplicate memories (same agent + scope + content).
        let merged = Self::merge_exact_duplicates(&conn)?;

        // Commit only after both decay and merge succeed; any earlier `?`
        // return drops `tx` and rolls back the whole cycle.
        tx.commit()
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ConsolidationReport {
            memories_merged: merged,
            memories_decayed: decayed as u64,
            duration_ms,
        })
    }

    /// Collapse exact-duplicate memories within the same `(agent_id, scope)`.
    ///
    /// Memories with byte-identical `content` are deduplicated: the
    /// highest-confidence row survives (tie-break: most recently accessed,
    /// then id for determinism) and the rest are soft-deleted. The survivor
    /// absorbs the group's total `access_count`, the maximum `confidence`, and
    /// the latest `accessed_at` so popularity/recency isn't lost. Returns the
    /// number of memories merged away (soft-deleted).
    ///
    /// Exact-match only — semantic near-duplicate merging would need embedding
    /// comparison and risks false merges, so it's intentionally out of scope.
    fn merge_exact_duplicates(conn: &Connection) -> RustyHandResult<u64> {
        // Find groups with more than one live row sharing the same content.
        let mut group_stmt = conn
            .prepare(
                "SELECT agent_id, scope, content FROM memories
                 WHERE deleted = 0
                 GROUP BY agent_id, scope, content
                 HAVING COUNT(*) > 1",
            )
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let group_iter = group_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let groups: Vec<(String, String, String)> = group_iter
            .collect::<Result<_, _>>()
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        drop(group_stmt);

        let mut merged = 0u64;
        for (agent_id, scope, content) in groups {
            // Survivor first: highest confidence, then most recent access, then id.
            let mut stmt = conn
                .prepare(
                    "SELECT id, confidence, access_count, accessed_at FROM memories
                     WHERE deleted = 0 AND agent_id = ?1 AND scope = ?2 AND content = ?3
                     ORDER BY confidence DESC, accessed_at DESC, id ASC",
                )
                .map_err(|e| RustyHandError::Memory(e.to_string()))?;
            let row_iter = stmt
                .query_map(rusqlite::params![agent_id, scope, content], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, f64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|e| RustyHandError::Memory(e.to_string()))?;
            let rows: Vec<(String, f64, i64, String)> = row_iter
                .collect::<Result<_, _>>()
                .map_err(|e| RustyHandError::Memory(e.to_string()))?;
            drop(stmt);

            if rows.len() < 2 {
                continue;
            }

            let survivor_id = rows[0].0.clone();
            let max_conf = rows.iter().map(|r| r.1).fold(f64::MIN, f64::max);
            let total_access: i64 = rows.iter().map(|r| r.2).sum();
            let latest_accessed = rows.iter().map(|r| r.3.clone()).max().unwrap_or_default();

            conn.execute(
                "UPDATE memories SET confidence = ?1, access_count = ?2, accessed_at = ?3
                 WHERE id = ?4",
                rusqlite::params![max_conf, total_access, latest_accessed, survivor_id],
            )
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;

            for loser in &rows[1..] {
                conn.execute(
                    "UPDATE memories SET deleted = 1 WHERE id = ?1",
                    rusqlite::params![loser.0],
                )
                .map_err(|e| RustyHandError::Memory(e.to_string()))?;
                merged += 1;
            }
        }

        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::run_migrations;

    fn setup() -> ConsolidationEngine {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        ConsolidationEngine::new(Arc::new(Mutex::new(conn)), 0.1)
    }

    #[test]
    fn test_consolidation_empty() {
        let engine = setup();
        let report = engine.consolidate().unwrap();
        assert_eq!(report.memories_decayed, 0);
    }

    #[test]
    fn test_consolidation_decays_old_memories() {
        let engine = setup();
        let conn = engine.conn.lock().unwrap();
        // Insert an old memory
        let old_date = (Utc::now() - chrono::Duration::days(30)).to_rfc3339();
        conn.execute(
            "INSERT INTO memories (id, agent_id, content, source, scope, confidence, metadata, created_at, accessed_at, access_count, deleted)
             VALUES ('test-id', 'agent-1', 'old memory', '\"conversation\"', 'episodic', 0.9, '{}', ?1, ?1, 0, 0)",
            rusqlite::params![old_date],
        ).unwrap();
        drop(conn);

        let report = engine.consolidate().unwrap();
        assert_eq!(report.memories_decayed, 1);

        // Verify confidence was reduced
        let conn = engine.conn.lock().unwrap();
        let confidence: f64 = conn
            .query_row(
                "SELECT confidence FROM memories WHERE id = 'test-id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(confidence < 0.9);
    }

    #[test]
    fn test_consolidation_merges_exact_duplicates() {
        let engine = setup();
        let now = Utc::now().to_rfc3339();
        let older = (Utc::now() - chrono::Duration::days(1)).to_rfc3339();
        {
            let conn = engine.conn.lock().unwrap();
            // Two identical-content memories for the same agent+scope (dup), plus
            // a distinct one and a same-content-but-different-scope one (no merge).
            let insert = |id: &str, scope: &str, content: &str, conf: f64, ac: i64, acc: &str| {
                conn.execute(
                    "INSERT INTO memories (id, agent_id, content, source, scope, confidence, metadata, created_at, accessed_at, access_count, deleted)
                     VALUES (?1, 'agent-1', ?2, '\"conversation\"', ?3, ?4, '{}', ?5, ?5, ?6, 0)",
                    rusqlite::params![id, content, scope, conf, acc, ac],
                ).unwrap();
            };
            insert("dup-low", "episodic", "the sky is blue", 0.4, 2, &older);
            insert("dup-high", "episodic", "the sky is blue", 0.8, 3, &now);
            insert("unique", "episodic", "grass is green", 0.5, 1, &now);
            insert("other-scope", "semantic", "the sky is blue", 0.7, 1, &now);
        }

        let report = engine.consolidate().unwrap();
        assert_eq!(report.memories_merged, 1, "one duplicate should be merged");

        let conn = engine.conn.lock().unwrap();
        // Survivor is the higher-confidence row; loser is soft-deleted.
        let low_deleted: i64 = conn
            .query_row(
                "SELECT deleted FROM memories WHERE id = 'dup-low'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(low_deleted, 1, "lower-confidence duplicate soft-deleted");
        let (high_deleted, high_access): (i64, i64) = conn
            .query_row(
                "SELECT deleted, access_count FROM memories WHERE id = 'dup-high'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(high_deleted, 0, "survivor stays live");
        assert_eq!(high_access, 5, "survivor absorbs summed access_count (3+2)");
        // Distinct content and different-scope same-content are untouched.
        let unique_deleted: i64 = conn
            .query_row(
                "SELECT deleted FROM memories WHERE id = 'unique'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let other_deleted: i64 = conn
            .query_row(
                "SELECT deleted FROM memories WHERE id = 'other-scope'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(unique_deleted, 0);
        assert_eq!(
            other_deleted, 0,
            "same content in a different scope is not merged"
        );
    }
}
