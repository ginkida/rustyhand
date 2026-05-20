//! LLM response cache — avoids duplicate API calls for identical prompts.
//!
//! Caches by SHA256(model + system_prompt + messages). Only caches
//! pure text responses (no tool_use). Configurable TTL and max entries.
//! Thread-safe via `DashMap`.

use crate::llm_driver::CompletionResponse;
use dashmap::DashMap;
use rusty_hand_types::message::{ContentBlock, Message, StopReason, TokenUsage};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

/// Write a length-prefixed byte slice into the hasher. Used by
/// `cache_key` to encode each field so user-controlled content can't
/// produce hash collisions via separator forging.
fn write_lp(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

/// Default cache TTL: 5 minutes.
const DEFAULT_TTL_SECS: u64 = 300;

/// Maximum cache entries to prevent unbounded memory growth.
const MAX_ENTRIES: usize = 1000;

/// A cached LLM response.
#[derive(Clone)]
struct CachedResponse {
    text: String,
    usage: TokenUsage,
    inserted_at: Instant,
}

/// Thread-safe in-memory LLM response cache.
pub struct LlmCache {
    entries: DashMap<String, CachedResponse>,
    ttl: Duration,
    enabled: bool,
}

impl LlmCache {
    /// Create a new cache. Pass `Duration::ZERO` to disable.
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: DashMap::new(),
            ttl,
            enabled: !ttl.is_zero(),
        }
    }

    /// Create a cache with the default TTL (5 minutes).
    pub fn default_ttl() -> Self {
        Self::new(Duration::from_secs(DEFAULT_TTL_SECS))
    }

    /// Create a disabled (passthrough) cache.
    pub fn disabled() -> Self {
        Self::new(Duration::ZERO)
    }

    /// Compute cache key from model + system prompt + messages.
    ///
    /// Each field is length-prefixed before hashing. The previous
    /// implementation joined `<role>:<content>;` per message with a
    /// bare `;` separator, which was a collision foothold for any
    /// user-controlled `content` that contained the separator pattern:
    ///
    /// ```text
    /// A = [User: "A;Assistant:B"]      → `User:A;Assistant:B;`
    /// B = [User: "A", Assistant: "B"]  → `User:A;Assistant:B;`
    /// ```
    ///
    /// Same hash, different conversations — A's cached response would
    /// be returned for B (and vice versa). Length-prefixing each field
    /// makes the encoding unambiguous: a `len` byte block cannot be
    /// confused with the content bytes.
    pub fn cache_key(model: &str, system: Option<&str>, messages: &[Message]) -> String {
        let mut hasher = Sha256::new();
        // Format-version prefix so a future hash-shape change can
        // invalidate at-rest entries cleanly (currently in-memory
        // only, but the discipline costs nothing).
        hasher.update([0x01_u8]);
        write_lp(&mut hasher, model.as_bytes());
        // Distinguish `None` system prompt from `Some("")` system
        // prompt with a 1-byte tag — previously both hashed identically.
        match system {
            None => hasher.update([0x00_u8]),
            Some(sys) => {
                hasher.update([0x01_u8]);
                write_lp(&mut hasher, sys.as_bytes());
            }
        }
        // Hash message roles and text content (skip images/tool blocks
        // for key stability). Length-prefix each field.
        hasher.update((messages.len() as u32).to_le_bytes());
        for msg in messages {
            let role_str = format!("{:?}", msg.role);
            write_lp(&mut hasher, role_str.as_bytes());
            let content_text = msg.content.text_content();
            write_lp(&mut hasher, content_text.as_bytes());
        }
        hex::encode(hasher.finalize())
    }

    /// Look up a cached response. Returns `None` if miss or expired.
    pub fn get(&self, key: &str) -> Option<CompletionResponse> {
        if !self.enabled {
            return None;
        }
        let entry = self.entries.get(key)?;
        if entry.inserted_at.elapsed() > self.ttl {
            drop(entry);
            self.entries.remove(key);
            return None;
        }
        Some(CompletionResponse {
            content: vec![ContentBlock::Text {
                text: entry.text.clone(),
            }],
            stop_reason: StopReason::EndTurn,
            tool_calls: vec![],
            usage: entry.usage,
        })
    }

    /// Store a response in the cache. Only caches pure text responses
    /// (no tool_use, stop_reason == EndTurn).
    pub fn put(&self, key: String, response: &CompletionResponse) {
        if !self.enabled {
            return;
        }
        // Only cache pure text responses — tool_use responses are dynamic
        if response.stop_reason != StopReason::EndTurn || !response.tool_calls.is_empty() {
            return;
        }
        let text = response.text();
        if text.is_empty() {
            return;
        }

        // Evict oldest entries if at capacity
        if self.entries.len() >= MAX_ENTRIES {
            // Find and remove the oldest entry
            let oldest = self
                .entries
                .iter()
                .min_by_key(|e| e.value().inserted_at)
                .map(|e| e.key().clone());
            if let Some(oldest_key) = oldest {
                self.entries.remove(&oldest_key);
            }
        }

        self.entries.insert(
            key,
            CachedResponse {
                text,
                usage: response.usage,
                inserted_at: Instant::now(),
            },
        );
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of cache hits (for metrics). Approximate — counts remaining entries.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Clear all cached entries.
    ///
    /// Intended for config hot-reload: when the default model or provider
    /// changes, previously cached responses belong to a different model
    /// and must be invalidated to avoid stale reads.
    pub fn clear(&self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_hand_types::message::Message;

    #[test]
    fn test_cache_hit() {
        let cache = LlmCache::new(Duration::from_secs(60));
        let key = LlmCache::cache_key("model-1", Some("system"), &[Message::user("hello")]);

        let response = CompletionResponse {
            content: vec![ContentBlock::Text {
                text: "Hi there!".to_string(),
            }],
            stop_reason: StopReason::EndTurn,
            tool_calls: vec![],
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
            },
        };

        cache.put(key.clone(), &response);
        let cached = cache.get(&key).unwrap();
        assert_eq!(cached.text(), "Hi there!");
        assert_eq!(cached.usage.total(), 15);
    }

    #[test]
    fn test_cache_miss() {
        let cache = LlmCache::new(Duration::from_secs(60));
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_skip_tool_use() {
        let cache = LlmCache::new(Duration::from_secs(60));
        let key = "tool-response".to_string();

        let response = CompletionResponse {
            content: vec![],
            stop_reason: StopReason::ToolUse,
            tool_calls: vec![rusty_hand_types::tool::ToolCall {
                id: "1".into(),
                name: "web_search".into(),
                input: serde_json::json!({}),
            }],
            usage: TokenUsage::default(),
        };

        cache.put(key.clone(), &response);
        assert!(cache.get(&key).is_none()); // Should not be cached
    }

    #[test]
    fn test_cache_disabled() {
        let cache = LlmCache::disabled();
        let key = "test".to_string();
        let response = CompletionResponse {
            content: vec![ContentBlock::Text {
                text: "cached".to_string(),
            }],
            stop_reason: StopReason::EndTurn,
            tool_calls: vec![],
            usage: TokenUsage::default(),
        };
        cache.put(key.clone(), &response);
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_cache_expiry() {
        let cache = LlmCache::new(Duration::from_millis(1));
        let key = "expire-me".to_string();
        let response = CompletionResponse {
            content: vec![ContentBlock::Text {
                text: "temp".to_string(),
            }],
            stop_reason: StopReason::EndTurn,
            tool_calls: vec![],
            usage: TokenUsage::default(),
        };
        cache.put(key.clone(), &response);
        std::thread::sleep(Duration::from_millis(5));
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_different_messages_different_keys() {
        let key1 = LlmCache::cache_key("m", None, &[Message::user("hello")]);
        let key2 = LlmCache::cache_key("m", None, &[Message::user("world")]);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_same_messages_same_key() {
        let key1 = LlmCache::cache_key("m", Some("s"), &[Message::user("hello")]);
        let key2 = LlmCache::cache_key("m", Some("s"), &[Message::user("hello")]);
        assert_eq!(key1, key2);
    }

    /// Regression: the previous cache_key encoded messages as
    /// `<role>:<content>;` joined together with no length prefix. A
    /// user message whose content embedded the role separator pattern
    /// produced the same hash as a multi-message conversation. The
    /// fix length-prefixes every field so this can never happen:
    ///
    ///   A = [User "Assistant:hi"]            → must NOT collide with
    ///   B = [User "", Assistant "hi"]        → different conversations
    #[test]
    fn cache_key_no_collision_from_separator_in_user_content() {
        let a = LlmCache::cache_key("model", None, &[Message::user("Assistant:hi")]);
        let b = LlmCache::cache_key(
            "model",
            None,
            &[Message::user(""), Message::assistant("hi")],
        );
        assert_ne!(
            a, b,
            "user content containing the role separator must not collide with a real multi-message conversation"
        );

        // And the same shape with the trailing `;` separator: pre-fix
        // these two also produced the same hash:
        //   A = [User "msg1;User:msg2"]
        //   B = [User "msg1", User "msg2"]
        let c = LlmCache::cache_key("model", None, &[Message::user("msg1;User:msg2")]);
        let d = LlmCache::cache_key(
            "model",
            None,
            &[Message::user("msg1"), Message::user("msg2")],
        );
        assert_ne!(
            c, d,
            "user content containing a `;User:` fragment must not collide with two separate user messages"
        );
    }

    /// Regression: pre-fix, `Some("")` (empty system prompt) and `None`
    /// (no system prompt) produced identical hash inputs:
    ///   None    → `model||...`
    ///   Some("") → `model||...`
    /// The fix tags the system field with 0x00/0x01 so the two states
    /// are distinguishable.
    #[test]
    fn cache_key_distinguishes_none_system_from_empty_system() {
        let none_sys = LlmCache::cache_key("model", None, &[Message::user("x")]);
        let empty_sys = LlmCache::cache_key("model", Some(""), &[Message::user("x")]);
        assert_ne!(
            none_sys, empty_sys,
            "None system prompt and Some(\"\") must produce distinct cache keys"
        );
    }
}
