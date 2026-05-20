//! Fallback driver — tries multiple LLM drivers in sequence.
//!
//! If the primary driver fails with a non-retryable error, the fallback driver
//! moves to the next driver in the chain.

use crate::llm_driver::{CompletionRequest, CompletionResponse, LlmDriver, LlmError, StreamEvent};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::warn;

/// A driver that wraps multiple LLM drivers and tries each in order.
///
/// On failure, moves to the next driver. Rate-limit and overload errors
/// are bubbled up for retry logic to handle.
pub struct FallbackDriver {
    drivers: Vec<Arc<dyn LlmDriver>>,
}

impl FallbackDriver {
    /// Create a new fallback driver from an ordered chain of drivers.
    ///
    /// The first driver is the primary; subsequent are fallbacks.
    pub fn new(drivers: Vec<Arc<dyn LlmDriver>>) -> Self {
        Self { drivers }
    }
}

#[async_trait]
impl LlmDriver for FallbackDriver {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let mut last_error = None;

        for (i, driver) in self.drivers.iter().enumerate() {
            match driver.complete(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(e @ LlmError::RateLimited { .. }) | Err(e @ LlmError::Overloaded { .. }) => {
                    // Retryable errors — bubble up for the retry loop to handle
                    return Err(e);
                }
                Err(e) => {
                    warn!(
                        driver_index = i,
                        error = %e,
                        "Fallback driver failed, trying next"
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::Api {
            status: 0,
            message: "No drivers configured in fallback chain".to_string(),
        }))
    }

    async fn stream(
        &self,
        request: CompletionRequest,
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<CompletionResponse, LlmError> {
        let mut last_error = None;

        for (i, driver) in self.drivers.iter().enumerate() {
            // Use a per-attempt intermediate channel so we can count how
            // many StreamEvents the driver actually relayed before
            // succeeding or erroring. If a driver streams partial output
            // and *then* fails, falling through to the next driver would
            // emit a Frankenstein response on `tx`: receiver concatenates
            // partial output from driver 1 with the full output of
            // driver 2. We surface the partial-stream failure verbatim
            // instead — agent_loop already handles upstream errors, and a
            // garbled mixed response is strictly worse than an honest
            // failure the caller can retry.
            let (inner_tx, mut inner_rx) = tokio::sync::mpsc::channel::<StreamEvent>(64);
            let outer_tx = tx.clone();
            let relayed = Arc::new(AtomicU64::new(0));
            let relayed_clone = relayed.clone();
            let relay = tokio::spawn(async move {
                while let Some(ev) = inner_rx.recv().await {
                    relayed_clone.fetch_add(1, Ordering::Relaxed);
                    if outer_tx.send(ev).await.is_err() {
                        // Downstream consumer gave up — stop relaying
                        // and let the driver finish on its own (we'll
                        // still see its terminal Result).
                        break;
                    }
                }
            });
            let result = driver.stream(request.clone(), inner_tx).await;
            // Wait for the relay to drain — `inner_tx` was moved into
            // the driver future and dropped when that future resolved,
            // so `inner_rx.recv()` returns None and the task ends.
            let _ = relay.await;
            let relayed_count = relayed.load(Ordering::Relaxed);

            match result {
                Ok(response) => return Ok(response),
                Err(e @ LlmError::RateLimited { .. }) | Err(e @ LlmError::Overloaded { .. }) => {
                    return Err(e);
                }
                Err(e) if relayed_count > 0 => {
                    // Partial output already on the wire — falling
                    // through would corrupt the receiver. Surface the
                    // error so the caller treats this as a stream
                    // failure rather than two LLMs' outputs concatenated.
                    warn!(
                        driver_index = i,
                        error = %e,
                        events_relayed = relayed_count,
                        "Fallback driver (stream) failed mid-stream — not falling through to avoid Frankenstein response"
                    );
                    return Err(e);
                }
                Err(e) => {
                    warn!(
                        driver_index = i,
                        error = %e,
                        "Fallback driver (stream) failed before any output, trying next"
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::Api {
            status: 0,
            message: "No drivers configured in fallback chain".to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_driver::CompletionResponse;
    use rusty_hand_types::message::{ContentBlock, StopReason, TokenUsage};

    struct FailDriver;

    #[async_trait]
    impl LlmDriver for FailDriver {
        async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            Err(LlmError::Api {
                status: 500,
                message: "Internal error".to_string(),
            })
        }
    }

    struct OkDriver;

    #[async_trait]
    impl LlmDriver for OkDriver {
        async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            Ok(CompletionResponse {
                content: vec![ContentBlock::Text {
                    text: "OK".to_string(),
                }],
                stop_reason: StopReason::EndTurn,
                tool_calls: vec![],
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                },
            })
        }
    }

    fn test_request() -> CompletionRequest {
        CompletionRequest {
            model: "test".to_string(),
            messages: vec![],
            tools: vec![],
            max_tokens: 100,
            temperature: 0.0,
            system: None,
            thinking: None,
            response_format: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_fallback_primary_succeeds() {
        let driver = FallbackDriver::new(vec![
            Arc::new(OkDriver) as Arc<dyn LlmDriver>,
            Arc::new(FailDriver) as Arc<dyn LlmDriver>,
        ]);
        let result = driver.complete(test_request()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().text(), "OK");
    }

    #[tokio::test]
    async fn test_fallback_primary_fails_secondary_succeeds() {
        let driver = FallbackDriver::new(vec![
            Arc::new(FailDriver) as Arc<dyn LlmDriver>,
            Arc::new(OkDriver) as Arc<dyn LlmDriver>,
        ]);
        let result = driver.complete(test_request()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fallback_all_fail() {
        let driver = FallbackDriver::new(vec![
            Arc::new(FailDriver) as Arc<dyn LlmDriver>,
            Arc::new(FailDriver) as Arc<dyn LlmDriver>,
        ]);
        let result = driver.complete(test_request()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limit_bubbles_up() {
        struct RateLimitDriver;

        #[async_trait]
        impl LlmDriver for RateLimitDriver {
            async fn complete(
                &self,
                _req: CompletionRequest,
            ) -> Result<CompletionResponse, LlmError> {
                Err(LlmError::RateLimited {
                    retry_after_ms: 5000,
                })
            }
        }

        let driver = FallbackDriver::new(vec![
            Arc::new(RateLimitDriver) as Arc<dyn LlmDriver>,
            Arc::new(OkDriver) as Arc<dyn LlmDriver>,
        ]);
        let result = driver.complete(test_request()).await;
        // Rate limit should NOT fall through to next driver
        assert!(matches!(result, Err(LlmError::RateLimited { .. })));
    }

    /// Driver that sends N text-delta events on the stream tx and
    /// then returns an error — simulates a mid-stream failure (network
    /// drop, malformed SSE frame, etc).
    struct PartialThenFailDriver {
        chunk: &'static str,
    }

    #[async_trait]
    impl LlmDriver for PartialThenFailDriver {
        async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            Err(LlmError::Api {
                status: 0,
                message: "complete not implemented for streaming-only test driver".into(),
            })
        }
        async fn stream(
            &self,
            _req: CompletionRequest,
            tx: tokio::sync::mpsc::Sender<StreamEvent>,
        ) -> Result<CompletionResponse, LlmError> {
            tx.send(StreamEvent::TextDelta {
                text: self.chunk.to_string(),
            })
            .await
            .ok();
            Err(LlmError::Api {
                status: 500,
                message: "mid-stream failure".into(),
            })
        }
    }

    /// Driver that streams a single complete chunk + finalises OK.
    struct FullStreamDriver {
        chunk: &'static str,
    }

    #[async_trait]
    impl LlmDriver for FullStreamDriver {
        async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
            Err(LlmError::Api {
                status: 0,
                message: "complete not implemented for streaming-only test driver".into(),
            })
        }
        async fn stream(
            &self,
            _req: CompletionRequest,
            tx: tokio::sync::mpsc::Sender<StreamEvent>,
        ) -> Result<CompletionResponse, LlmError> {
            tx.send(StreamEvent::TextDelta {
                text: self.chunk.to_string(),
            })
            .await
            .ok();
            Ok(CompletionResponse {
                content: vec![ContentBlock::Text {
                    text: self.chunk.to_string(),
                }],
                stop_reason: StopReason::EndTurn,
                tool_calls: vec![],
                usage: TokenUsage {
                    input_tokens: 1,
                    output_tokens: 1,
                },
            })
        }
    }

    /// Regression: FallbackDriver::stream used to clone `tx` and pass
    /// it to every driver in sequence. If driver 1 streamed partial
    /// output before erroring, driver 2's full stream was appended on
    /// top — the receiver concatenated "Hello " + "World response" into
    /// "Hello World response", a Frankenstein answer mixing two LLMs.
    /// The fix counts events relayed per attempt: if a driver sent any
    /// events before failing, the FallbackDriver surfaces the error
    /// rather than falling through.
    #[tokio::test]
    async fn stream_does_not_fall_through_after_partial_output() {
        let driver = FallbackDriver::new(vec![
            Arc::new(PartialThenFailDriver { chunk: "Hello " }) as Arc<dyn LlmDriver>,
            Arc::new(FullStreamDriver {
                chunk: "World response",
            }) as Arc<dyn LlmDriver>,
        ]);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let result = driver.stream(test_request(), tx).await;
        assert!(
            result.is_err(),
            "mid-stream failure must surface as Err, not silent fallthrough"
        );

        // Collect everything the receiver saw. It should only contain
        // the first driver's partial chunk — NOT both drivers' output.
        let mut received = String::new();
        while let Ok(ev) =
            tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await
        {
            match ev {
                Some(StreamEvent::TextDelta { text }) => received.push_str(&text),
                Some(_) => {} // other event kinds ignored for this test
                None => break,
            }
        }
        assert_eq!(
            received, "Hello ",
            "receiver must not see Frankenstein output (got `{received}`)"
        );
    }

    /// Companion case: if driver 1 errors *before* sending any event,
    /// the FallbackDriver SHOULD fall through to driver 2. Only mid-
    /// stream failures are unrecoverable.
    #[tokio::test]
    async fn stream_falls_through_when_no_partial_output() {
        struct InstantFailDriver;
        #[async_trait]
        impl LlmDriver for InstantFailDriver {
            async fn complete(
                &self,
                _req: CompletionRequest,
            ) -> Result<CompletionResponse, LlmError> {
                Err(LlmError::Api {
                    status: 401,
                    message: "auth".into(),
                })
            }
            async fn stream(
                &self,
                _req: CompletionRequest,
                _tx: tokio::sync::mpsc::Sender<StreamEvent>,
            ) -> Result<CompletionResponse, LlmError> {
                // Returns immediately, no events relayed.
                Err(LlmError::Api {
                    status: 401,
                    message: "auth".into(),
                })
            }
        }

        let driver = FallbackDriver::new(vec![
            Arc::new(InstantFailDriver) as Arc<dyn LlmDriver>,
            Arc::new(FullStreamDriver { chunk: "OK" }) as Arc<dyn LlmDriver>,
        ]);
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        let result = driver.stream(test_request(), tx).await;
        assert!(
            result.is_ok(),
            "instant-fail in driver 1 must allow fallthrough to driver 2"
        );

        let mut received = String::new();
        while let Ok(ev) =
            tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await
        {
            match ev {
                Some(StreamEvent::TextDelta { text }) => received.push_str(&text),
                Some(_) => {}
                None => break,
            }
        }
        assert_eq!(received, "OK");
    }
}
