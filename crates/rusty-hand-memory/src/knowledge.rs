//! Knowledge graph backed by SQLite.
//!
//! Stores entities and relations with support for graph pattern queries.

use chrono::Utc;
use rusqlite::Connection;
use rusty_hand_types::error::{RustyHandError, RustyHandResult};
use rusty_hand_types::memory::{
    Entity, EntityType, GraphMatch, GraphPattern, Relation, RelationType,
};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Upper bound on edges returned from a single graph query (single- or
/// multi-hop), to bound response size and memory.
const MAX_GRAPH_MATCHES: usize = 500;

/// Upper bound on entities materialized by `list_entities`, to bound memory on
/// large graphs.
const MAX_GRAPH_ENTITIES: usize = 1000;

/// Knowledge graph store backed by SQLite.
#[derive(Clone)]
pub struct KnowledgeStore {
    conn: Arc<Mutex<Connection>>,
}

impl KnowledgeStore {
    /// Create a new knowledge store wrapping the given connection.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Add an entity to the knowledge graph.
    pub fn add_entity(&self, entity: Entity) -> RustyHandResult<String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Internal(e.to_string()))?;
        let id = if entity.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            entity.id.clone()
        };
        let entity_type_str = serde_json::to_string(&entity.entity_type)
            .map_err(|e| RustyHandError::Serialization(e.to_string()))?;
        let props_str = serde_json::to_string(&entity.properties)
            .map_err(|e| RustyHandError::Serialization(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO entities (id, entity_type, name, properties, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(id) DO UPDATE SET name = ?3, properties = ?4, updated_at = ?5",
            rusqlite::params![id, entity_type_str, entity.name, props_str, now],
        )
        .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        Ok(id)
    }

    /// Add a relation between two entities.
    pub fn add_relation(&self, relation: Relation) -> RustyHandResult<String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Internal(e.to_string()))?;
        let id = if relation.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            relation.id.clone()
        };
        let rel_type_str = serde_json::to_string(&relation.relation)
            .map_err(|e| RustyHandError::Serialization(e.to_string()))?;
        let props_str = serde_json::to_string(&relation.properties)
            .map_err(|e| RustyHandError::Serialization(e.to_string()))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO relations (id, source_entity, relation_type, target_entity, properties, confidence, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id,
                relation.source,
                rel_type_str,
                relation.target,
                props_str,
                relation.confidence as f64,
                now,
            ],
        )
        .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        Ok(id)
    }

    /// Update an existing relation in-place. Only the fields present in
    /// the provided Relation are written. Returns true when a row was
    /// updated, false when no relation with that id exists.
    pub fn update_relation(&self, id: &str, relation: Relation) -> RustyHandResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Internal(e.to_string()))?;
        let rel_type_str = serde_json::to_string(&relation.relation)
            .map_err(|e| RustyHandError::Serialization(e.to_string()))?;
        let props_str = serde_json::to_string(&relation.properties)
            .map_err(|e| RustyHandError::Serialization(e.to_string()))?;
        let affected = conn
            .execute(
                "UPDATE relations SET source_entity = ?1, relation_type = ?2,
                    target_entity = ?3, properties = ?4, confidence = ?5
                 WHERE id = ?6",
                rusqlite::params![
                    relation.source,
                    rel_type_str,
                    relation.target,
                    props_str,
                    relation.confidence as f64,
                    id,
                ],
            )
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        Ok(affected > 0)
    }

    /// Remove a relation by its ID. Returns true if a row was deleted.
    pub fn remove_relation(&self, id: &str) -> RustyHandResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Internal(e.to_string()))?;
        let affected = conn
            .execute("DELETE FROM relations WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        Ok(affected > 0)
    }

    /// Remove an entity by ID. Cascade-deletes any relations referencing it.
    /// Returns true if the entity row was deleted.
    pub fn remove_entity(&self, id: &str) -> RustyHandResult<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM relations WHERE source_entity = ?1 OR target_entity = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let affected = conn
            .execute("DELETE FROM entities WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        Ok(affected > 0)
    }

    /// List entities in the knowledge graph (capped at [`MAX_GRAPH_ENTITIES`]
    /// to bound memory — an unbounded `SELECT` materializes the entire table on
    /// every render of the graph view).
    pub fn list_entities(&self) -> RustyHandResult<Vec<Entity>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, entity_type, name, properties, created_at, updated_at \
                 FROM entities ORDER BY name LIMIT {MAX_GRAPH_ENTITIES}"
            ))
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let entity_type_str: String = row.get(1)?;
                let name: String = row.get(2)?;
                let props_str: String = row.get::<_, String>(3).unwrap_or_default();
                let created_at_str: String = row.get::<_, String>(4).unwrap_or_default();
                let updated_at_str: String = row.get::<_, String>(5).unwrap_or_default();
                Ok((
                    id,
                    entity_type_str,
                    name,
                    props_str,
                    created_at_str,
                    updated_at_str,
                ))
            })
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let mut entities = Vec::new();
        for row in rows {
            let (id, entity_type_str, name, props_str, created_at_str, updated_at_str) =
                row.map_err(|e| RustyHandError::Memory(e.to_string()))?;
            let entity_type: EntityType = serde_json::from_str::<EntityType>(&entity_type_str)
                .unwrap_or(EntityType::Custom(entity_type_str));
            let properties: std::collections::HashMap<String, serde_json::Value> =
                serde_json::from_str(&props_str).unwrap_or_default();
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            entities.push(Entity {
                id,
                entity_type,
                name,
                properties,
                created_at,
                updated_at,
            });
        }
        Ok(entities)
    }

    /// Total number of relations (edges) in the graph — for reporting the true
    /// edge count when a query result is capped at [`MAX_GRAPH_MATCHES`].
    pub fn count_relations(&self) -> RustyHandResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Internal(e.to_string()))?;
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        Ok(n.max(0) as usize)
    }

    /// Query the knowledge graph with a pattern.
    ///
    /// Honors `pattern.max_depth`: depth <= 1 (or no `source` anchor) does a
    /// single-hop direct match; depth > 1 does a breadth-first neighborhood
    /// expansion outward from `source`, following the same `relation` filter at
    /// each hop. Previously `max_depth` was silently ignored, so advertised
    /// multi-hop traversal only ever returned direct (1-hop) edges. The result
    /// is capped at [`MAX_GRAPH_MATCHES`] to bound memory.
    pub fn query_graph(&self, pattern: GraphPattern) -> RustyHandResult<Vec<GraphMatch>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| RustyHandError::Internal(e.to_string()))?;

        let relation_json = match pattern.relation.as_ref() {
            Some(r) => Some(
                serde_json::to_string(r)
                    .map_err(|e| RustyHandError::Serialization(e.to_string()))?,
            ),
            None => None,
        };

        let max_depth = pattern.max_depth.max(1);
        // Single-hop: direct pattern match honoring all three filters. Also the
        // path when there is no `source` anchor to expand outward from.
        if max_depth <= 1 || pattern.source.is_none() {
            return Self::fetch_hop(
                &conn,
                pattern.source.as_deref(),
                relation_json.as_deref(),
                pattern.target.as_deref(),
                MAX_GRAPH_MATCHES,
            );
        }

        // Multi-hop BFS neighborhood from `source`. Expand `relation`-filtered
        // edges hop-by-hop up to max_depth; `target` (if any) is applied as a
        // post-filter on the endpoint so it doesn't prune intermediate hops.
        let mut matches: Vec<GraphMatch> = Vec::new();
        let mut seen_edges: HashSet<String> = HashSet::new();
        let mut visited_nodes: HashSet<String> = HashSet::new();
        if let Some(s) = pattern.source.as_deref() {
            visited_nodes.insert(s.to_string());
        }

        let seed = Self::fetch_hop(
            &conn,
            pattern.source.as_deref(),
            relation_json.as_deref(),
            None,
            MAX_GRAPH_MATCHES,
        )?;
        let mut frontier: Vec<String> = Vec::new();
        for m in seed {
            if seen_edges.insert(m.relation.id.clone()) {
                frontier.push(m.target.id.clone());
                matches.push(m);
            }
        }

        let mut hop = 1u32;
        while hop < max_depth && matches.len() < MAX_GRAPH_MATCHES {
            let mut next_frontier = Vec::new();
            for node in std::mem::take(&mut frontier) {
                if !visited_nodes.insert(node.clone()) {
                    continue;
                }
                if matches.len() >= MAX_GRAPH_MATCHES {
                    break;
                }
                let edges = Self::fetch_hop(
                    &conn,
                    Some(&node),
                    relation_json.as_deref(),
                    None,
                    MAX_GRAPH_MATCHES,
                )?;
                for m in edges {
                    if matches.len() >= MAX_GRAPH_MATCHES {
                        break;
                    }
                    if seen_edges.insert(m.relation.id.clone()) {
                        next_frontier.push(m.target.id.clone());
                        matches.push(m);
                    }
                }
            }
            if next_frontier.is_empty() {
                break;
            }
            frontier = next_frontier;
            hop += 1;
        }

        if let Some(target) = pattern.target.as_deref() {
            matches.retain(|m| m.target.id == target || m.target.name == target);
        }

        Ok(matches)
    }

    /// Fetch a single hop of graph edges matching the given filters.
    fn fetch_hop(
        conn: &Connection,
        source: Option<&str>,
        relation_json: Option<&str>,
        target: Option<&str>,
        limit: usize,
    ) -> RustyHandResult<Vec<GraphMatch>> {
        let mut sql = String::from(
            "SELECT
                s.id, s.entity_type, s.name, s.properties, s.created_at, s.updated_at,
                r.id, r.source_entity, r.relation_type, r.target_entity, r.properties, r.confidence, r.created_at,
                t.id, t.entity_type, t.name, t.properties, t.created_at, t.updated_at
             FROM relations r
             JOIN entities s ON r.source_entity = s.id
             JOIN entities t ON r.target_entity = t.id
             WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut idx = 1;

        if let Some(source) = source {
            sql.push_str(&format!(" AND (s.id = ?{idx} OR s.name = ?{idx})"));
            params.push(Box::new(source.to_string()));
            idx += 1;
        }
        if let Some(relation_json) = relation_json {
            sql.push_str(&format!(" AND r.relation_type = ?{idx}"));
            params.push(Box::new(relation_json.to_string()));
            idx += 1;
        }
        if let Some(target) = target {
            sql.push_str(&format!(" AND (t.id = ?{idx} OR t.name = ?{idx})"));
            params.push(Box::new(target.to_string()));
            let _ = idx;
        }

        sql.push_str(&format!(" LIMIT {limit}"));

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();

        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(RawGraphRow {
                    s_id: row.get(0)?,
                    s_type: row.get(1)?,
                    s_name: row.get(2)?,
                    s_props: row.get(3)?,
                    s_created: row.get(4)?,
                    s_updated: row.get(5)?,
                    r_id: row.get(6)?,
                    r_source: row.get(7)?,
                    r_type: row.get(8)?,
                    r_target: row.get(9)?,
                    r_props: row.get(10)?,
                    r_confidence: row.get(11)?,
                    r_created: row.get(12)?,
                    t_id: row.get(13)?,
                    t_type: row.get(14)?,
                    t_name: row.get(15)?,
                    t_props: row.get(16)?,
                    t_created: row.get(17)?,
                    t_updated: row.get(18)?,
                })
            })
            .map_err(|e| RustyHandError::Memory(e.to_string()))?;

        let mut matches = Vec::new();
        for row_result in rows {
            let r = row_result.map_err(|e| RustyHandError::Memory(e.to_string()))?;
            matches.push(GraphMatch {
                source: parse_entity(
                    &r.s_id,
                    &r.s_type,
                    &r.s_name,
                    &r.s_props,
                    &r.s_created,
                    &r.s_updated,
                ),
                relation: parse_relation(
                    &r.r_id,
                    &r.r_source,
                    &r.r_type,
                    &r.r_target,
                    &r.r_props,
                    r.r_confidence,
                    &r.r_created,
                ),
                target: parse_entity(
                    &r.t_id,
                    &r.t_type,
                    &r.t_name,
                    &r.t_props,
                    &r.t_created,
                    &r.t_updated,
                ),
            });
        }
        Ok(matches)
    }
}

/// Raw row from a graph query.
struct RawGraphRow {
    s_id: String,
    s_type: String,
    s_name: String,
    s_props: String,
    s_created: String,
    s_updated: String,
    r_id: String,
    r_source: String,
    r_type: String,
    r_target: String,
    r_props: String,
    r_confidence: f64,
    r_created: String,
    t_id: String,
    t_type: String,
    t_name: String,
    t_props: String,
    t_created: String,
    t_updated: String,
}

fn parse_entity(
    id: &str,
    etype: &str,
    name: &str,
    props: &str,
    created: &str,
    updated: &str,
) -> Entity {
    let entity_type: EntityType =
        serde_json::from_str(etype).unwrap_or(EntityType::Custom("unknown".to_string()));
    let properties: HashMap<String, serde_json::Value> =
        serde_json::from_str(props).unwrap_or_default();
    let created_at = chrono::DateTime::parse_from_rfc3339(created)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let updated_at = chrono::DateTime::parse_from_rfc3339(updated)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    Entity {
        id: id.to_string(),
        entity_type,
        name: name.to_string(),
        properties,
        created_at,
        updated_at,
    }
}

fn parse_relation(
    id: &str,
    source: &str,
    rtype: &str,
    target: &str,
    props: &str,
    confidence: f64,
    created: &str,
) -> Relation {
    let relation: RelationType = serde_json::from_str(rtype).unwrap_or(RelationType::RelatedTo);
    let properties: HashMap<String, serde_json::Value> =
        serde_json::from_str(props).unwrap_or_default();
    let created_at = chrono::DateTime::parse_from_rfc3339(created)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    Relation {
        id: id.to_string(),
        source: source.to_string(),
        relation,
        target: target.to_string(),
        properties,
        confidence: confidence as f32,
        created_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::run_migrations;

    fn setup() -> KnowledgeStore {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        KnowledgeStore::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn test_add_and_query_entity() {
        let store = setup();
        let id = store
            .add_entity(Entity {
                id: String::new(),
                entity_type: EntityType::Person,
                name: "Alice".to_string(),
                properties: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .unwrap();
        assert!(!id.is_empty());
    }

    #[test]
    fn test_add_relation_and_query() {
        let store = setup();
        let alice_id = store
            .add_entity(Entity {
                id: "alice".to_string(),
                entity_type: EntityType::Person,
                name: "Alice".to_string(),
                properties: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .unwrap();
        let company_id = store
            .add_entity(Entity {
                id: "acme".to_string(),
                entity_type: EntityType::Organization,
                name: "Acme Corp".to_string(),
                properties: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .unwrap();
        store
            .add_relation(Relation {
                id: String::new(),
                source: alice_id.clone(),
                relation: RelationType::WorksAt,
                target: company_id,
                properties: HashMap::new(),
                confidence: 0.95,
                created_at: Utc::now(),
            })
            .unwrap();

        let matches = store
            .query_graph(GraphPattern {
                source: Some(alice_id),
                relation: Some(RelationType::WorksAt),
                target: None,
                max_depth: 1,
            })
            .unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].target.name, "Acme Corp");
    }

    #[test]
    fn test_query_graph_honors_max_depth_multi_hop() {
        // A -RelatedTo-> B -RelatedTo-> C. A depth-1 query from A returns only
        // the A→B edge; a depth-2 query must also traverse B→C (previously
        // max_depth was ignored and only direct edges were ever returned).
        let store = setup();
        let mk = |id: &str| {
            store
                .add_entity(Entity {
                    id: id.to_string(),
                    entity_type: EntityType::Concept,
                    name: id.to_string(),
                    properties: HashMap::new(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
                .unwrap()
        };
        let a = mk("A");
        let _b = mk("B");
        let _c = mk("C");
        let edge = |s: &str, t: &str| {
            store
                .add_relation(Relation {
                    id: String::new(),
                    source: s.to_string(),
                    relation: RelationType::RelatedTo,
                    target: t.to_string(),
                    properties: HashMap::new(),
                    confidence: 1.0,
                    created_at: Utc::now(),
                })
                .unwrap();
        };
        edge("A", "B");
        edge("B", "C");

        let depth1 = store
            .query_graph(GraphPattern {
                source: Some(a.clone()),
                relation: Some(RelationType::RelatedTo),
                target: None,
                max_depth: 1,
            })
            .unwrap();
        assert_eq!(depth1.len(), 1, "depth-1 returns only the direct A→B edge");

        let depth2 = store
            .query_graph(GraphPattern {
                source: Some(a),
                relation: Some(RelationType::RelatedTo),
                target: None,
                max_depth: 2,
            })
            .unwrap();
        assert_eq!(depth2.len(), 2, "depth-2 must also traverse B→C");
        assert!(
            depth2.iter().any(|m| m.target.name == "C"),
            "depth-2 result must include node C"
        );
    }
}
