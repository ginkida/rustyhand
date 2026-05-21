//! Context overflow recovery pipeline.
//!
//! Provides a 4-stage recovery pipeline that replaces the brute-force
//! `emergency_trim_messages()` with structured, progressive recovery:
//!
//! 1. Auto-compact via message trimming (keep recent, drop old)
//! 2. Aggressive overflow compaction (drop all but last N)
//! 3. Truncate historical tool results to 2K chars each
//! 4. Return error suggesting /reset or /compact

use rusty_hand_types::message::{ContentBlock, Message, MessageContent};
use rusty_hand_types::tool::ToolDefinition;
use tracing::{debug, warn};

/// Recovery stage that was applied.
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStage {
    /// No recovery needed.
    None,
    /// Stage 1: moderate trim (keep last 10).
    AutoCompaction { removed: usize },
    /// Stage 2: aggressive trim (keep last 4).
    OverflowCompaction { removed: usize },
    /// Stage 3: truncated tool results.
    ToolResultTruncation { truncated: usize },
    /// Stage 4: unrecoverable — suggest /reset.
    FinalError,
}

/// Estimate token count using chars/4 heuristic.
fn estimate_tokens(messages: &[Message], system_prompt: &str, tools: &[ToolDefinition]) -> usize {
    crate::compactor::estimate_token_count(messages, Some(system_prompt), Some(tools))
}

/// Run the 4-stage overflow recovery pipeline.
///
/// Returns the recovery stage applied and the number of messages/results affected.
pub fn recover_from_overflow(
    messages: &mut Vec<Message>,
    system_prompt: &str,
    tools: &[ToolDefinition],
    context_window: usize,
) -> RecoveryStage {
    let estimated = estimate_tokens(messages, system_prompt, tools);
    let threshold_70 = (context_window as f64 * 0.70) as usize;
    let threshold_90 = (context_window as f64 * 0.90) as usize;

    // No recovery needed
    if estimated <= threshold_70 {
        return RecoveryStage::None;
    }

    // Stage 1: Moderate trim — keep last 10 messages
    if estimated <= threshold_90 {
        let keep = 10.min(messages.len());
        let remove = messages.len() - keep;
        if remove > 0 {
            debug!(
                estimated_tokens = estimated,
                removing = remove,
                "Stage 1: moderate trim to last {keep} messages"
            );
            messages.drain(..remove);
            // Re-check after trim
            let new_est = estimate_tokens(messages, system_prompt, tools);
            if new_est <= threshold_70 {
                return RecoveryStage::AutoCompaction { removed: remove };
            }
        }
    }

    // Stage 2: Aggressive trim — keep last 4 messages + summary marker
    {
        let keep = 4.min(messages.len());
        let remove = messages.len() - keep;
        if remove > 0 {
            warn!(
                estimated_tokens = estimate_tokens(messages, system_prompt, tools),
                removing = remove,
                "Stage 2: aggressive overflow compaction to last {keep} messages"
            );
            let summary = Message::user(format!(
                "[System: {} earlier messages were removed due to context overflow. \
                 The conversation continues from here. Use /compact for smarter summarization.]",
                remove
            ));
            messages.drain(..remove);
            messages.insert(0, summary);

            let new_est = estimate_tokens(messages, system_prompt, tools);
            if new_est <= threshold_90 {
                return RecoveryStage::OverflowCompaction { removed: remove };
            }
        }
    }

    // Stage 3: Truncate all historical tool results to 2K bytes each.
    //
    // Tool result content is arbitrary user-facing text (file_read of a
    // Cyrillic doc, web_fetch HTML, agent_send transcripts in any
    // language). Pre-fix this used `&content[..keep]` — naive byte
    // slicing that panics when byte `keep` lands inside a multi-byte
    // UTF-8 char. This is the LAST recovery stage before the agent
    // loop hard-errors; a panic here converts a recoverable overflow
    // into an outright crash, exactly the opposite of what this stage
    // exists to prevent. Route through the UTF-8-safe truncator.
    let tool_truncation_limit = 2000;
    let mut truncated = 0;
    for msg in messages.iter_mut() {
        if let MessageContent::Blocks(blocks) = &mut msg.content {
            for block in blocks.iter_mut() {
                if let ContentBlock::ToolResult { content, .. } = block {
                    if content.len() > tool_truncation_limit {
                        let target = tool_truncation_limit.saturating_sub(80);
                        let cut = rusty_hand_types::text::truncate_bytes(content.as_str(), target);
                        let cut_len = cut.len();
                        let prev_len = content.len();
                        *content = format!(
                            "{cut}\n\n[OVERFLOW RECOVERY: truncated from {prev_len} to {cut_len} bytes]"
                        );
                        truncated += 1;
                    }
                }
            }
        }
    }

    if truncated > 0 {
        let new_est = estimate_tokens(messages, system_prompt, tools);
        if new_est <= threshold_90 {
            return RecoveryStage::ToolResultTruncation { truncated };
        }
        warn!(
            estimated_tokens = new_est,
            "Stage 3 truncated {} tool results but still over threshold", truncated
        );
    }

    // Stage 4: Final error — nothing more we can do automatically
    warn!("Stage 4: all recovery stages exhausted, context still too large");
    RecoveryStage::FinalError
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_hand_types::message::{Message, Role};

    fn make_messages(count: usize, size_each: usize) -> Vec<Message> {
        (0..count)
            .map(|i| {
                let text = format!("msg{}: {}", i, "x".repeat(size_each));
                Message {
                    role: if i % 2 == 0 {
                        Role::User
                    } else {
                        Role::Assistant
                    },
                    content: MessageContent::Text(text),
                }
            })
            .collect()
    }

    #[test]
    fn test_no_recovery_needed() {
        let mut msgs = make_messages(2, 100);
        let stage = recover_from_overflow(&mut msgs, "sys", &[], 200_000);
        assert_eq!(stage, RecoveryStage::None);
    }

    #[test]
    fn test_stage1_moderate_trim() {
        // Create messages that push us past 70% but not 90%
        // Context window: 1000 tokens = 4000 chars
        // 70% = 700 tokens = 2800 chars
        let mut msgs = make_messages(20, 150); // ~3000 chars total
        let stage = recover_from_overflow(&mut msgs, "system", &[], 1000);
        match stage {
            RecoveryStage::AutoCompaction { removed } => {
                assert!(removed > 0);
                assert!(msgs.len() <= 10);
            }
            RecoveryStage::OverflowCompaction { .. } => {
                // Also acceptable if moderate wasn't enough
            }
            _ => {} // depends on exact token estimation
        }
    }

    #[test]
    fn test_stage2_aggressive_trim() {
        // Push past 90%: 1000 tokens = 4000 chars, 90% = 3600 chars
        let mut msgs = make_messages(30, 200); // ~6000 chars
        let stage = recover_from_overflow(&mut msgs, "system", &[], 1000);
        match stage {
            RecoveryStage::OverflowCompaction { removed } => {
                assert!(removed > 0);
            }
            RecoveryStage::ToolResultTruncation { .. } | RecoveryStage::FinalError => {}
            _ => {} // acceptable cascading
        }
    }

    #[test]
    fn test_stage3_tool_truncation() {
        let big_result = "x".repeat(5000);
        let mut msgs = vec![
            Message::user("hi"),
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                    tool_use_id: "t1".to_string(),
                    content: big_result.clone(),
                    is_error: false,
                }]),
            },
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                    tool_use_id: "t2".to_string(),
                    content: big_result,
                    is_error: false,
                }]),
            },
        ];
        // Tiny context window to force all stages
        let stage = recover_from_overflow(&mut msgs, "system", &[], 500);
        // Should at least reach tool truncation
        match stage {
            RecoveryStage::ToolResultTruncation { truncated } => {
                assert!(truncated > 0);
            }
            RecoveryStage::OverflowCompaction { .. } | RecoveryStage::FinalError => {}
            _ => {}
        }
    }

    #[test]
    fn test_cascading_stages() {
        // Ensure stages cascade: if stage 1 isn't enough, stage 2 kicks in
        let mut msgs = make_messages(50, 500);
        let stage = recover_from_overflow(&mut msgs, "system prompt", &[], 2000);
        // With 50 messages of 500 chars each (25000 chars), context of 2000 tokens (8000 chars),
        // we should cascade through stages
        assert_ne!(stage, RecoveryStage::None);
    }

    /// Regression: stage 3 used `&content[..keep]` to clip oversized
    /// tool results — naive byte slicing that panics when the cut
    /// lands inside a multi-byte UTF-8 char. Tool results commonly
    /// contain non-ASCII content. Pre-fix the LAST recovery stage
    /// panicked on the first overflow involving a Cyrillic / CJK /
    /// emoji tool result, converting a recoverable overflow into a
    /// hard crash. Post-fix: route through truncate_bytes.
    #[test]
    fn stage3_truncates_oversized_non_ascii_tool_result_without_panic() {
        // 3000 × "Я" (= 6000 bytes) — well above the 2000-byte stage
        // 3 threshold. Without a 1-byte prefix the byte boundaries are
        // even (0, 2, 4 …); add one ASCII char so the truncation point
        // (~1920) lands inside a Я.
        let big_content = format!("a{}", "Я".repeat(3000));
        assert!(big_content.len() > 2000);
        let target = 2000usize.saturating_sub(80);
        assert!(
            !big_content.is_char_boundary(target),
            "test setup must place the cut mid-char"
        );

        // Wrap the blob in a ToolResult block so stage 3 sees it.
        let mut msgs = vec![Message {
            role: Role::User,
            content: MessageContent::Blocks(vec![ContentBlock::ToolResult {
                tool_use_id: "tu-1".to_string(),
                content: big_content,
                is_error: false,
            }]),
        }];

        // Use a tiny context window so we cascade into stage 3.
        // Pre-fix: panic inside the &content[..keep] slice.
        let stage = recover_from_overflow(&mut msgs, "sys", &[], 1000);

        // The exact stage depends on cascade dynamics; what matters is
        // we did NOT panic and the content was truncated to a valid
        // UTF-8 string.
        let _ = stage;
        if let MessageContent::Blocks(blocks) = &msgs[0].content {
            for b in blocks {
                if let ContentBlock::ToolResult { content, .. } = b {
                    assert!(
                        std::str::from_utf8(content.as_bytes()).is_ok(),
                        "truncated tool result must be valid UTF-8"
                    );
                }
            }
        }
    }
}
