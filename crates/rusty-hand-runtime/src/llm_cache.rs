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
    pub fn cache_key(model: &str, system: Option<&str>, messages: &[Message]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(model.as_bytes());
        hasher.update(b"|");
        if let Some(sys) = system {
            hasher.update(sys.as_bytes());
        }
        hasher.update(b"|");
        // Hash message roles and text content (skip images/tool blocks for key stability)
        for msg in messages {
            hasher.update(format!("{:?}:", msg.role).as_bytes());
            hasher.update(msg.content.text_content().as_bytes());
            hasher.update(b";");
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
}
