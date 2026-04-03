//! Lenient serde deserializers for backwards-compatible agent manifest loading.
//!
//! When agent manifests are stored as msgpack blobs in SQLite, schema changes
//! (e.g., a field changing from integer to struct, or from map to Vec) cause
//! hard deserialization failures. These helpers gracefully return defaults
//! for type-mismatched fields instead of failing the entire deserialization.

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;

/// Deserialize a `Vec<T>` leniently: if the stored value is not a sequence
/// (e.g., it's a map, integer, string, bool, or null), return an empty Vec
/// instead of failing.
pub fn vec_lenient<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct VecLenientVisitor<T>(PhantomData<T>);

    impl<'de, T: Deserialize<'de>> Visitor<'de> for VecLenientVisitor<T> {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence (or any value, which will default to empty Vec)")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut vec = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            while let Some(item) = seq.next_element()? {
                vec.push(item);
            }
            Ok(vec)
        }

        // All non-sequence types return empty Vec
        fn visit_map<A>(self, mut _map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            // Drain the map to keep the deserializer state consistent
            while let Some((_, _)) = _map.next_entry::<de::IgnoredAny, de::IgnoredAny>()? {}
            Ok(Vec::new())
        }

        fn visit_i64<E: de::Error>(self, _v: i64) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }

        fn visit_u64<E: de::Error>(self, _v: u64) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }

        fn visit_f64<E: de::Error>(self, _v: f64) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }

        fn visit_str<E: de::Error>(self, _v: &str) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }

        fn visit_bool<E: de::Error>(self, _v: bool) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }
    }

    deserializer.deserialize_any(VecLenientVisitor(PhantomData))
}

/// Deserialize a `HashMap<K, V>` leniently: if the stored value is not a map
/// (e.g., it's a sequence, integer, string, bool, or null), return an empty
/// HashMap instead of failing.
pub fn map_lenient<'de, D, K, V>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Eq + Hash,
    V: Deserialize<'de>,
{
    struct MapLenientVisitor<K, V>(PhantomData<(K, V)>);

    impl<'de, K, V> Visitor<'de> for MapLenientVisitor<K, V>
    where
        K: Deserialize<'de> + Eq + Hash,
        V: Deserialize<'de>,
    {
        type Value = HashMap<K, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map (or any value, which will default to empty HashMap)")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut result = HashMap::with_capacity(map.size_hint().unwrap_or(0));
            while let Some((k, v)) = map.next_entry()? {
                result.insert(k, v);
            }
            Ok(result)
        }

        // All non-map types return empty HashMap
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            // Drain the sequence to keep the deserializer state consistent
            while seq.next_element::<de::IgnoredAny>()?.is_some() {}
            Ok(HashMap::new())
        }

        fn visit_i64<E: de::Error>(self, _v: i64) -> Result<Self::Value, E> {
            Ok(HashMap::new())
        }

        fn visit_u64<E: de::Error>(self, _v: u64) -> Result<Self::Value, E> {
            Ok(HashMap::new())
        }

        fn visit_f64<E: de::Error>(self, _v: f64) -> Result<Self::Value, E> {
            Ok(HashMap::new())
        }

        fn visit_str<E: de::Error>(self, _v: &str) -> Result<Self::Value, E> {
            Ok(HashMap::new())
        }

        fn visit_bool<E: de::Error>(self, _v: bool) -> Result<Self::Value, E> {
            Ok(HashMap::new())
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(HashMap::new())
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(HashMap::new())
        }
    }

    deserializer.deserialize_any(MapLenientVisitor(PhantomData))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestVec {
        #[serde(default, deserialize_with = "vec_lenient")]
        items: Vec<String>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestMap {
        #[serde(default, deserialize_with = "map_lenient")]
        items: HashMap<String, i32>,
    }

    // --- vec_lenient tests ---

    #[test]
    fn vec_lenient_accepts_sequence() {
        let json = r#"{"items": ["a", "b", "c"]}"#;
        let result: TestVec = serde_json::from_str(json).unwrap();
        assert_eq!(result.items, vec!["a", "b", "c"]);
    }

    #[test]
    fn vec_lenient_given_map_returns_empty() {
        let json = r#"{"items": {"key": "value"}}"#;
        let result: TestVec = serde_json::from_str(json).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn vec_lenient_given_integer_returns_empty() {
        let json = r#"{"items": 268435456}"#;
        let result: TestVec = serde_json::from_str(json).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn vec_lenient_given_string_returns_empty() {
        let json = r#"{"items": "not a vec"}"#;
        let result: TestVec = serde_json::from_str(json).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn vec_lenient_given_bool_returns_empty() {
        let json = r#"{"items": true}"#;
        let result: TestVec = serde_json::from_str(json).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn vec_lenient_given_null_returns_empty() {
        let json = r#"{"items": null}"#;
        let result: TestVec = serde_json::from_str(json).unwrap();
        assert!(result.items.is_empty());
    }

    // --- map_lenient tests ---

    #[test]
    fn map_lenient_accepts_map() {
        let json = r#"{"items": {"a": 1, "b": 2}}"#;
        let result: TestMap = serde_json::from_str(json).unwrap();
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items["a"], 1);
        assert_eq!(result.items["b"], 2);
    }

    #[test]
    fn map_lenient_given_sequence_returns_empty() {
        let json = r#"{"items": [1, 2, 3]}"#;
        let result: TestMap = serde_json::from_str(json).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn map_lenient_given_integer_returns_empty() {
        let json = r#"{"items": 42}"#;
        let result: TestMap = serde_json::from_str(json).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn map_lenient_given_string_returns_empty() {
        let json = r#"{"items": "not a map"}"#;
        let result: TestMap = serde_json::from_str(json).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn map_lenient_given_bool_returns_empty() {
        let json = r#"{"items": false}"#;
        let result: TestMap = serde_json::from_str(json).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn map_lenient_given_null_returns_empty() {
        let json = r#"{"items": null}"#;
        let result: TestMap = serde_json::from_str(json).unwrap();
        assert!(result.items.is_empty());
    }

    // --- msgpack round-trip test (simulates the actual agent manifest scenario) ---

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct OldManifest {
        name: String,
        fallback_models: u64,            // old format: integer
        skills: HashMap<String, String>, // old format: map
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct NewManifest {
        name: String,
        #[serde(default, deserialize_with = "vec_lenient")]
        fallback_models: Vec<String>, // new format: Vec
        #[serde(default, deserialize_with = "vec_lenient")]
        skills: Vec<String>, // new format: Vec
    }

    #[test]
    fn msgpack_old_format_deserializes_leniently() {
        // Serialize with the OLD schema
        let old = OldManifest {
            name: "test-agent".to_string(),
            fallback_models: 268435456,
            skills: {
                let mut m = HashMap::new();
                m.insert("web-search".to_string(), "enabled".to_string());
                m
            },
        };
        let blob = rmp_serde::to_vec_named(&old).unwrap();

        // Deserialize with the NEW schema — should succeed with empty defaults
        let new: NewManifest = rmp_serde::from_slice(&blob).unwrap();
        assert_eq!(new.name, "test-agent");
        assert!(new.fallback_models.is_empty());
        assert!(new.skills.is_empty());
    }
}
