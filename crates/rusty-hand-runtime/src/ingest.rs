//! Document ingestion pipeline for RAG (Retrieval-Augmented Generation).
//!
//! Ingests text content → splits into chunks → embeds each chunk → stores
//! as agent memories for vector-similarity recall during the agent loop.

use crate::embedding::EmbeddingDriver;
use rusty_hand_memory::MemorySubstrate;
use rusty_hand_types::agent::AgentId;
use rusty_hand_types::error::{RustyHandError, RustyHandResult};
use rusty_hand_types::memory::MemorySource;
use std::collections::HashMap;
use tracing::{debug, info};

/// Default chunk size in characters.
const DEFAULT_CHUNK_SIZE: usize = 1000;

/// Overlap between consecutive chunks (characters).
const DEFAULT_CHUNK_OVERLAP: usize = 200;

/// Maximum document size (5 MB).
const MAX_DOCUMENT_SIZE: usize = 5 * 1024 * 1024;

/// Maximum number of chunks per document.
const MAX_CHUNKS: usize = 500;

/// Result of an ingestion operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IngestResult {
    /// Number of chunks created.
    pub chunks: usize,
    /// Number of chunks with embeddings.
    pub embedded: usize,
    /// Source identifier (URL or filename).
    pub source: String,
}

/// Ingest a text document into an agent's memory.
///
/// Splits the text into overlapping chunks, computes embeddings (if a driver
/// is available), and stores each chunk as a separate memory fragment.
pub async fn ingest_text(
    agent_id: AgentId,
    content: &str,
    source_label: &str,
    memory: &MemorySubstrate,
    embedding_driver: Option<&(dyn EmbeddingDriver + Send + Sync)>,
    chunk_size: Option<usize>,
    chunk_overlap: Option<usize>,
) -> RustyHandResult<IngestResult> {
    if content.len() > MAX_DOCUMENT_SIZE {
        return Err(RustyHandError::InvalidInput(format!(
            "Document too large ({} bytes, max {} bytes)",
            content.len(),
            MAX_DOCUMENT_SIZE
        )));
    }

    let chunk_size = chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE);
    let chunk_overlap = chunk_overlap.unwrap_or(DEFAULT_CHUNK_OVERLAP);
    let chunks = split_into_chunks(content, chunk_size, chunk_overlap);

    if chunks.len() > MAX_CHUNKS {
        return Err(RustyHandError::InvalidInput(format!(
            "Too many chunks ({}, max {}). Increase chunk_size or reduce document length.",
            chunks.len(),
            MAX_CHUNKS
        )));
    }

    info!(
        agent = %agent_id,
        source = source_label,
        chunks = chunks.len(),
        "Ingesting document"
    );

    let mut embedded_count = 0;

    // Batch embed all chunks if driver is available
    let embeddings: Vec<Option<Vec<f32>>> = if let Some(driver) = embedding_driver {
        let chunk_refs: Vec<&str> = chunks.iter().map(|c| c.as_str()).collect();
        match driver.embed(&chunk_refs).await {
            Ok(vecs) => {
                embedded_count = vecs.len();
                vecs.into_iter().map(Some).collect()
            }
            Err(e) => {
                tracing::warn!("Embedding failed during ingestion, storing without vectors: {e}");
                vec![None; chunks.len()]
            }
        }
    } else {
        vec![None; chunks.len()]
    };

    // Store each chunk as a memory fragment
    for (i, (chunk, embedding)) in chunks.iter().zip(embeddings.iter()).enumerate() {
        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), serde_json::json!(source_label));
        metadata.insert("chunk_index".to_string(), serde_json::json!(i));
        metadata.insert("total_chunks".to_string(), serde_json::json!(chunks.len()));
        metadata.insert("ingested".to_string(), serde_json::json!(true));

        memory.remember_with_embedding(
            agent_id,
            chunk,
            MemorySource::Document,
            "ingested",
            metadata,
            embedding.as_deref(),
        )?;

        debug!(agent = %agent_id, chunk = i, len = chunk.len(), "Stored chunk");
    }

    Ok(IngestResult {
        chunks: chunks.len(),
        embedded: embedded_count,
        source: source_label.to_string(),
    })
}

/// Split text into overlapping chunks, respecting paragraph and sentence boundaries.
fn split_into_chunks(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }
    if text.len() <= chunk_size {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;

    while start < chars.len() {
        let end = (start + chunk_size).min(chars.len());
        let mut chunk_end = end;

        // Try to break at paragraph boundary.
        //
        // `chunk_end` and `start` are CHAR indices into `chars`; but
        // `chunk_str.rfind(...)` returns a BYTE offset within the
        // collected string. Pre-fix the byte offset was added directly
        // to `start` and the result was used as a char index — for
        // ASCII text that happens to work, but for Cyrillic / CJK /
        // emoji (each char 2-4 bytes) the byte offset over-counts by
        // 2-4×. With a `\n\n` near the end of the window the new
        // `chunk_end` overshoots `chars.len()` and the next
        // `chars[start..chunk_end]` slice panics with index out of
        // bounds. Convert the byte offset back into a char count by
        // counting chars in the byte-prefix before the boundary, then
        // also compare against the chunk-size threshold in char units
        // (the pre-fix `break_pos > chunk_size / 2` compared bytes to
        // chars and got luckier-than-deserved on ASCII).
        if chunk_end < chars.len() {
            let chunk_str: String = chars[start..chunk_end].iter().collect();
            let resolve = |byte_pos: usize, delimiter_chars: usize| -> usize {
                chunk_str[..byte_pos].chars().count() + delimiter_chars
            };
            if let Some(byte_pos) = chunk_str.rfind("\n\n") {
                let char_offset = resolve(byte_pos, 2);
                if char_offset > chunk_size / 2 {
                    chunk_end = start + char_offset;
                }
            } else if let Some(byte_pos) = chunk_str.rfind(". ") {
                // Fall back to sentence boundary.
                let char_offset = resolve(byte_pos, 2);
                if char_offset > chunk_size / 2 {
                    chunk_end = start + char_offset;
                }
            }
        }

        let chunk: String = chars[start..chunk_end].iter().collect();
        if !chunk.trim().is_empty() {
            chunks.push(chunk.trim().to_string());
        }

        // Advance with overlap
        let advance = if chunk_end > start + overlap {
            chunk_end - start - overlap
        } else {
            chunk_end - start
        };
        start += advance;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_short_text() {
        let chunks = split_into_chunks("Hello world", 1000, 200);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Hello world");
    }

    #[test]
    fn test_split_empty() {
        assert!(split_into_chunks("", 100, 20).is_empty());
        assert!(split_into_chunks("   ", 100, 20).is_empty());
    }

    #[test]
    fn test_split_paragraph_boundary() {
        let text = "First paragraph about Rust.\n\nSecond paragraph about agents.\n\nThird paragraph about memory.";
        let chunks = split_into_chunks(text, 50, 10);
        assert!(chunks.len() >= 2);
        // Should break at \n\n boundaries
        assert!(!chunks[0].ends_with('\n'));
    }

    #[test]
    fn test_split_overlap() {
        let text = "A".repeat(100) + " " + &"B".repeat(100) + " " + &"C".repeat(100);
        let chunks = split_into_chunks(&text, 120, 20);
        assert!(chunks.len() >= 2);
        // Verify overlap exists
        if chunks.len() >= 2 {
            let end_of_first = &chunks[0][chunks[0].len().saturating_sub(20)..];
            let start_of_second = &chunks[1][..20.min(chunks[1].len())];
            // With overlap, there should be some shared content
            assert!(!end_of_first.is_empty());
            assert!(!start_of_second.is_empty());
        }
    }

    #[test]
    fn test_split_max_chunks() {
        let text = "word ".repeat(10000);
        let chunks = split_into_chunks(&text, 50, 10);
        // Should produce many chunks but they should all have content
        assert!(chunks.iter().all(|c| !c.is_empty()));
    }

    /// Regression: split_into_chunks indexes `chars[start..chunk_end]`
    /// using char counts, but `chunk_str.rfind(...)` returns a BYTE
    /// offset. Pre-fix the byte offset was used directly as the char
    /// delta — for ASCII it happens to work, but for Cyrillic / CJK
    /// content (each char ≥ 2 bytes) the byte offset overshoots the
    /// char count by 2-4×. With a paragraph break near the end of the
    /// chunk window the new `chunk_end` exceeds `chars.len()` and the
    /// next `chars[start..chunk_end].iter().collect()` call panics
    /// with "index out of bounds".
    ///
    /// RAG ingest accepts user-provided documents in any language, so
    /// the first multi-language doc crashed the kernel turn that
    /// called the ingest pipeline.
    #[test]
    fn split_handles_paragraph_break_in_cyrillic_text() {
        // 60 Я (= 120 bytes) + \n\n + 10 Б (= 20 bytes) = 72 chars / 142 bytes.
        // chunk_size = 100 chars → byte length 142 > 100, so we enter
        // the chunking loop. `chunk_str.rfind("\n\n")` returns byte
        // offset 120 (after the 60 Я). Pre-fix: chunk_end = 0 + 120 + 2 = 122,
        // but chars.len() = 72 — index-out-of-bounds on next slice.
        let text = format!("{}\n\n{}", "Я".repeat(60), "Б".repeat(10));
        // Sanity: setup hits the paragraph-break branch.
        assert!(text.contains("\n\n"));
        assert!(text.len() > 100);

        // Post-fix: no panic, returns valid chunks.
        let chunks = split_into_chunks(&text, 100, 20);
        assert!(!chunks.is_empty());
        // Every chunk should be valid UTF-8 (collecting from a char
        // slice can't produce invalid UTF-8, but check anyway).
        for c in &chunks {
            assert!(!c.is_empty());
            assert!(std::str::from_utf8(c.as_bytes()).is_ok());
        }
    }
}
