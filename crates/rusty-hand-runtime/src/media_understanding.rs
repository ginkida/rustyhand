//! Media understanding engine — image description, audio transcription, video analysis.
//!
//! Auto-cascades through available providers based on configured API keys.

use rusty_hand_types::media::{
    MediaAttachment, MediaConfig, MediaSource, MediaType, MediaUnderstanding,
};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::info;

/// Media understanding engine.
pub struct MediaEngine {
    config: MediaConfig,
    semaphore: Arc<Semaphore>,
}

impl MediaEngine {
    pub fn new(config: MediaConfig) -> Self {
        let max = config.max_concurrency.clamp(1, 8);
        Self {
            config,
            semaphore: Arc::new(Semaphore::new(max)),
        }
    }

    /// Describe an image using a vision-capable LLM via the provider's
    /// Messages/Chat API (real implementation — sends an `Image` content block
    /// plus a describe prompt and returns the model's text).
    ///
    /// Provider is taken from `config.image_provider` or auto-detected
    /// (`ANTHROPIC_API_KEY`). The Anthropic driver encodes the image as a
    /// base64 source block; the OpenAI-compat driver encodes it as an
    /// `image_url` data URL — so anthropic/kimi and openrouter all work.
    /// Gemini/OpenAI were removed as direct providers in v0.7.0; reach those
    /// vision models via OpenRouter.
    pub async fn describe_image(
        &self,
        attachment: &MediaAttachment,
    ) -> Result<MediaUnderstanding, String> {
        attachment.validate()?;
        if attachment.media_type != MediaType::Image {
            return Err("Expected image attachment".into());
        }

        // Determine which provider to use
        let provider = self
            .config
            .image_provider
            .as_deref()
            .or_else(|| detect_vision_provider())
            .ok_or(
                "No vision-capable LLM provider configured. Set ANTHROPIC_API_KEY, \
                    or set OPENROUTER_API_KEY to reach Anthropic/OpenAI/Gemini vision \
                    models via OpenRouter (OpenAI/Gemini were removed as direct \
                    providers in v0.7.0)",
            )?;

        // Bound concurrent media calls (the engine's whole purpose).
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|e| format!("media engine semaphore closed: {e}"))?;

        // Resolve the source to base64 + media type for the image content block.
        let (data_base64, media_type) = resolve_image_base64(attachment).await?;

        let model = default_vision_model(provider).to_string();

        // Build a vision-capable driver. Both the Anthropic driver (base64
        // image blocks) and the OpenAI-compat driver (image_url data URLs)
        // serialize ContentBlock::Image, so anthropic/kimi and openrouter all
        // work. Passing api_key=None lets create_driver read the provider's
        // conventional env var itself.
        let driver = crate::drivers::create_driver(&crate::llm_driver::DriverConfig {
            provider: provider.to_string(),
            api_key: None,
            base_url: None,
        })
        .map_err(|e| format!("Failed to create vision driver for '{provider}': {e}"))?;

        let request = crate::llm_driver::CompletionRequest {
            model: model.clone(),
            messages: vec![rusty_hand_types::message::Message {
                role: rusty_hand_types::message::Role::User,
                content: rusty_hand_types::message::MessageContent::Blocks(vec![
                    rusty_hand_types::message::ContentBlock::Image {
                        media_type: media_type.clone(),
                        data: data_base64,
                    },
                    rusty_hand_types::message::ContentBlock::Text {
                        text: "Describe this image in detail. Note any text, people, objects, \
                               charts, UI elements, and notable context. Be concise but complete."
                            .to_string(),
                    },
                ]),
            }],
            tools: vec![],
            max_tokens: 1024,
            temperature: 0.2,
            system: Some(
                "You are a precise image-description assistant. Describe only what is actually \
                 visible; do not speculate beyond the image."
                    .to_string(),
            ),
            thinking: None,
            response_format: Default::default(),
        };

        let response = driver
            .complete(request)
            .await
            .map_err(|e| format!("Vision request to '{provider}' failed: {e}"))?;
        let description = response.text();
        if description.trim().is_empty() {
            return Err(format!("'{provider}' returned an empty image description"));
        }

        info!(provider, model = %model, "image described via vision LLM");
        Ok(MediaUnderstanding {
            media_type: MediaType::Image,
            description,
            provider: provider.to_string(),
            model,
        })
    }

    /// Transcribe audio using speech-to-text.
    /// Auto-cascade: Groq (whisper-large-v3-turbo) -> OpenAI (whisper-1).
    pub async fn transcribe_audio(
        &self,
        attachment: &MediaAttachment,
    ) -> Result<MediaUnderstanding, String> {
        attachment.validate()?;
        if attachment.media_type != MediaType::Audio {
            return Err("Expected audio attachment".into());
        }

        let provider = self
            .config
            .audio_provider
            .as_deref()
            .or_else(|| detect_audio_provider())
            .ok_or(
                "No audio transcription provider configured. Set GROQ_API_KEY or OPENAI_API_KEY",
            )?;

        let _permit = self.semaphore.acquire().await.map_err(|e| e.to_string())?;

        // Derive a proper filename with extension from mime_type
        // (Whisper APIs require an extension to detect format)
        let ext = match attachment.mime_type.as_str() {
            "audio/wav" => "wav",
            "audio/mpeg" | "audio/mp3" => "mp3",
            "audio/ogg" => "ogg",
            "audio/webm" => "webm",
            "audio/mp4" | "audio/m4a" => "m4a",
            "audio/flac" => "flac",
            _ => "wav",
        };

        // Read audio bytes from source
        let audio_bytes = match &attachment.source {
            MediaSource::FilePath { path } => tokio::fs::read(path)
                .await
                .map_err(|e| format!("Failed to read audio file '{}': {}", path, e))?,
            MediaSource::Base64 { data, .. } => {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .map_err(|e| format!("Failed to decode base64 audio: {}", e))?
            }
            MediaSource::Url { url } => {
                return Err(format!(
                    "URL-based audio source not supported for transcription: {}",
                    url
                ));
            }
        };
        let filename = format!("audio.{}", ext);

        let model = default_audio_model(provider);

        // Build API request
        let (api_url, api_key) = match provider {
            "groq" => (
                "https://api.groq.com/openai/v1/audio/transcriptions",
                std::env::var("GROQ_API_KEY").map_err(|_| "GROQ_API_KEY not set")?,
            ),
            "openai" => (
                "https://api.openai.com/v1/audio/transcriptions",
                std::env::var("OPENAI_API_KEY").map_err(|_| "OPENAI_API_KEY not set")?,
            ),
            other => return Err(format!("Unsupported audio provider: {}", other)),
        };

        info!(provider, model, filename = %filename, size = audio_bytes.len(), "Sending audio for transcription");

        let file_part = reqwest::multipart::Part::bytes(audio_bytes)
            .file_name(filename)
            .mime_str(&attachment.mime_type)
            .map_err(|e| format!("Failed to set MIME type: {}", e))?;

        let form = reqwest::multipart::Form::new()
            .part("file", file_part)
            .text("model", model.to_string())
            .text("response_format", "text");

        let client = crate::http_client::shared();
        let resp = client
            .post(api_url)
            .bearer_auth(&api_key)
            .multipart(form)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("Transcription request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {e}>"));
            return Err(format!("Transcription API error ({}): {}", status, body));
        }

        let transcription = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read transcription response: {}", e))?;

        let transcription = transcription.trim().to_string();
        if transcription.is_empty() {
            return Err("Transcription returned empty text".into());
        }

        info!(
            provider,
            model,
            chars = transcription.len(),
            "Audio transcription complete"
        );

        Ok(MediaUnderstanding {
            media_type: MediaType::Audio,
            description: transcription,
            provider: provider.to_string(),
            model: model.to_string(),
        })
    }

    /// Describe video using Gemini.
    pub async fn describe_video(
        &self,
        attachment: &MediaAttachment,
    ) -> Result<MediaUnderstanding, String> {
        attachment.validate()?;
        if attachment.media_type != MediaType::Video {
            return Err("Expected video attachment".into());
        }

        if !self.config.video_description {
            return Err(
                "Video description is disabled in configuration (set [media] \
                 video_description = true to attempt it)"
                    .into(),
            );
        }

        // Video understanding has no working backend in this build: the direct
        // video model (Gemini) was removed in v0.7.0 and the path was never
        // rewired. Fail fast with ONE honest message — don't gate on a
        // GEMINI/GOOGLE key first (setting one wouldn't help, which only sends
        // the user on a wild goose chase), and don't pretend the video was
        // understood. Point at the working alternative.
        Err(
            "Video description is not yet implemented in this build (the video model backend \
             was removed in v0.7.0 and not rewired). Extract a keyframe and use media_describe \
             (image vision via ANTHROPIC_API_KEY or OPENROUTER_API_KEY) instead."
                .into(),
        )
    }

    /// Process multiple attachments concurrently (bounded by max_concurrency).
    pub async fn process_attachments(
        &self,
        attachments: Vec<MediaAttachment>,
    ) -> Vec<Result<MediaUnderstanding, String>> {
        let mut handles = Vec::new();

        for attachment in attachments {
            let sem = self.semaphore.clone();
            let config = self.config.clone();
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.map_err(|e| e.to_string())?;
                let engine = MediaEngine {
                    config,
                    semaphore: Arc::new(Semaphore::new(1)), // inner engine, no extra semaphore
                };
                match attachment.media_type {
                    MediaType::Image => engine.describe_image(&attachment).await,
                    MediaType::Audio => engine.transcribe_audio(&attachment).await,
                    MediaType::Video => engine.describe_video(&attachment).await,
                }
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(format!("Task failed: {e}"))),
            }
        }
        results
    }
}

/// Detect which vision provider is available based on environment variables.
///
/// Returns provider IDs that are also valid for `create_driver()`. Pre-v0.7.0
/// this also returned "openai" and "gemini" — but those providers were
/// removed in v0.7.0, so auto-detecting them would just produce an
/// "Unknown provider" error downstream. Reach those models via OpenRouter
/// instead. Currently `anthropic` is the only direct provider in the
/// catalog with confirmed vision support.
fn detect_vision_provider() -> Option<&'static str> {
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        return Some("anthropic");
    }
    // OpenRouter reaches Anthropic/OpenAI/Gemini vision models over the
    // OpenAI-compatible wire; default_vision_model("openrouter") is wired,
    // so a user whose only key is OPENROUTER_API_KEY can still use vision.
    if std::env::var("OPENROUTER_API_KEY").is_ok() {
        return Some("openrouter");
    }
    None
}

/// Detect which audio transcription provider is available.
fn detect_audio_provider() -> Option<&'static str> {
    if std::env::var("GROQ_API_KEY").is_ok() {
        return Some("groq");
    }
    if std::env::var("OPENAI_API_KEY").is_ok() {
        return Some("openai");
    }
    None
}

/// Get the default vision model for a provider.
fn default_vision_model(provider: &str) -> &str {
    match provider {
        "anthropic" => "claude-sonnet-4-6",
        "openrouter" => "anthropic/claude-sonnet-4.6",
        "kimi" => "kimi-for-coding",
        // Removed as direct providers in v0.7.0 — kept for lookup completeness.
        "openai" => "gpt-4o",
        "gemini" => "gemini-2.5-flash",
        _ => "unknown",
    }
}

/// Resolve a media attachment's source to `(base64_data, media_type)` for use
/// as an inline image content block. Handles base64 (pass-through), local file
/// (read + encode), and URL (SSRF-checked fetch + encode).
async fn resolve_image_base64(attachment: &MediaAttachment) -> Result<(String, String), String> {
    use base64::Engine;
    let fallback_mime = attachment.mime_type.clone();
    match &attachment.source {
        MediaSource::Base64 { data, mime_type } => {
            let mt = if mime_type.is_empty() {
                fallback_mime
            } else {
                mime_type.clone()
            };
            Ok((data.clone(), mt))
        }
        MediaSource::FilePath { path } => {
            let bytes = tokio::fs::read(path)
                .await
                .map_err(|e| format!("Failed to read image file '{}': {}", path, e))?;
            Ok((
                base64::engine::general_purpose::STANDARD.encode(&bytes),
                fallback_mime,
            ))
        }
        MediaSource::Url { url } => {
            // Fail closed on SSRF (private/metadata/userinfo bypass) before fetching.
            crate::web_fetch::check_ssrf(url)?;
            let resp = crate::http_client::shared()
                .get(url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch image URL '{}': {}", url, e))?;
            let detected = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(';').next().unwrap_or(s).trim().to_string())
                .filter(|s| s.starts_with("image/"));
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| format!("Failed to read image bytes from '{}': {}", url, e))?;
            Ok((
                base64::engine::general_purpose::STANDARD.encode(&bytes),
                detected.unwrap_or(fallback_mime),
            ))
        }
    }
}

/// Get the default audio model for a provider.
fn default_audio_model(provider: &str) -> &str {
    match provider {
        "groq" => "whisper-large-v3-turbo",
        "openai" => "whisper-1",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_hand_types::media::{MediaSource, MAX_IMAGE_BYTES};

    #[test]
    fn test_engine_creation() {
        let config = MediaConfig::default();
        let engine = MediaEngine::new(config);
        assert_eq!(engine.config.max_concurrency, 2);
    }

    #[test]
    fn test_engine_max_concurrency_clamped() {
        let config = MediaConfig {
            max_concurrency: 100,
            ..Default::default()
        };
        let engine = MediaEngine::new(config);
        // Semaphore was clamped to 8
        assert!(engine.semaphore.available_permits() <= 8);
    }

    #[tokio::test]
    async fn test_describe_image_wrong_type() {
        let engine = MediaEngine::new(MediaConfig::default());
        let attachment = MediaAttachment {
            media_type: MediaType::Audio,
            mime_type: "audio/mpeg".into(),
            source: MediaSource::FilePath {
                path: "test.mp3".into(),
            },
            size_bytes: 1024,
        };
        let result = engine.describe_image(&attachment).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Expected image"));
    }

    #[tokio::test]
    async fn test_describe_image_invalid_mime() {
        let engine = MediaEngine::new(MediaConfig::default());
        let attachment = MediaAttachment {
            media_type: MediaType::Image,
            mime_type: "application/pdf".into(),
            source: MediaSource::FilePath {
                path: "test.pdf".into(),
            },
            size_bytes: 1024,
        };
        let result = engine.describe_image(&attachment).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_describe_image_too_large() {
        let engine = MediaEngine::new(MediaConfig::default());
        let attachment = MediaAttachment {
            media_type: MediaType::Image,
            mime_type: "image/png".into(),
            source: MediaSource::FilePath {
                path: "big.png".into(),
            },
            size_bytes: MAX_IMAGE_BYTES + 1,
        };
        let result = engine.describe_image(&attachment).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transcribe_audio_wrong_type() {
        let engine = MediaEngine::new(MediaConfig::default());
        let attachment = MediaAttachment {
            media_type: MediaType::Image,
            mime_type: "image/png".into(),
            source: MediaSource::FilePath {
                path: "test.png".into(),
            },
            size_bytes: 1024,
        };
        let result = engine.transcribe_audio(&attachment).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_video_disabled() {
        let config = MediaConfig {
            video_description: false,
            ..Default::default()
        };
        let engine = MediaEngine::new(config);
        let attachment = MediaAttachment {
            media_type: MediaType::Video,
            mime_type: "video/mp4".into(),
            source: MediaSource::FilePath {
                path: "test.mp4".into(),
            },
            size_bytes: 1024,
        };
        let result = engine.describe_video(&attachment).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("disabled"));
    }

    #[test]
    fn test_detect_vision_provider_none() {
        // In test env, likely no API keys set — should return None.
        // (This test is environment-dependent, but safe.)
        let _ = detect_vision_provider(); // Just verify it doesn't panic
    }

    #[test]
    fn test_default_vision_models() {
        assert_eq!(default_vision_model("anthropic"), "claude-sonnet-4-6");
        assert_eq!(default_vision_model("openai"), "gpt-4o");
        assert_eq!(default_vision_model("gemini"), "gemini-2.5-flash");
        assert_eq!(default_vision_model("unknown"), "unknown");
    }

    #[test]
    fn test_default_audio_models() {
        assert_eq!(default_audio_model("groq"), "whisper-large-v3-turbo");
        assert_eq!(default_audio_model("openai"), "whisper-1");
    }

    #[tokio::test]
    async fn test_transcribe_audio_rejects_image_type() {
        let engine = MediaEngine::new(MediaConfig::default());
        let attachment = MediaAttachment {
            media_type: MediaType::Image,
            mime_type: "image/png".into(),
            source: MediaSource::FilePath {
                path: "test.png".into(),
            },
            size_bytes: 1024,
        };
        let result = engine.transcribe_audio(&attachment).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Expected audio"));
    }

    #[tokio::test]
    async fn test_transcribe_audio_no_provider() {
        // With no API keys set, should fail with provider error
        let engine = MediaEngine::new(MediaConfig::default());
        let attachment = MediaAttachment {
            media_type: MediaType::Audio,
            mime_type: "audio/webm".into(),
            source: MediaSource::FilePath {
                path: "test.webm".into(),
            },
            size_bytes: 1024,
        };
        let result = engine.transcribe_audio(&attachment).await;
        // Either fails with "No audio transcription provider" or file read error
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_transcribe_audio_url_source_rejected() {
        // URL source should be rejected
        let config = MediaConfig {
            audio_provider: Some("groq".to_string()),
            ..Default::default()
        };
        let engine = MediaEngine::new(config);
        let attachment = MediaAttachment {
            media_type: MediaType::Audio,
            mime_type: "audio/mpeg".into(),
            source: MediaSource::Url {
                url: "https://example.com/audio.mp3".into(),
            },
            size_bytes: 1024,
        };
        let result = engine.transcribe_audio(&attachment).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("URL-based audio source not supported"));
    }

    #[tokio::test]
    async fn test_transcribe_audio_file_not_found() {
        let config = MediaConfig {
            audio_provider: Some("groq".to_string()),
            ..Default::default()
        };
        let engine = MediaEngine::new(config);
        let attachment = MediaAttachment {
            media_type: MediaType::Audio,
            mime_type: "audio/webm".into(),
            source: MediaSource::FilePath {
                path: "/nonexistent/path/audio.webm".into(),
            },
            size_bytes: 1024,
        };
        let result = engine.transcribe_audio(&attachment).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read audio file"));
    }

    /// Regression: `describe_image` used to return a fake placeholder
    /// success string (`[Image description would be generated by X
    /// provider]`) for valid images. The Telegram/Discord/Slack channel
    /// adapters auto-describe incoming photos with this method and
    /// leaked the placeholder into agent prompts, so agents responded
    /// confidently about images they had not seen. Fail loudly instead,
    /// so the adapter's existing `Err(_) => "Image received (description
    /// unavailable)"` fallback fires.
    #[tokio::test]
    async fn describe_image_now_processes_the_image_no_stub() {
        // Vision is implemented (v0.7.x elevation). With an explicit provider
        // and a non-existent file, the real path runs and fails at file read —
        // deterministic, no network, no API key. This proves the old
        // "not yet implemented" stub is gone AND that the image is actually
        // being resolved/processed rather than fake-described.
        let config = MediaConfig {
            image_provider: Some("anthropic".into()),
            ..Default::default()
        };
        let engine = MediaEngine::new(config);
        let attachment = MediaAttachment {
            media_type: MediaType::Image,
            mime_type: "image/png".into(),
            source: MediaSource::FilePath {
                path: "definitely-missing-fixture.png".into(),
            },
            size_bytes: 1024,
        };
        let err = engine
            .describe_image(&attachment)
            .await
            .expect_err("missing file must error during image resolution");
        assert!(
            err.contains("Failed to read image file"),
            "should fail resolving the image source, got: {err}"
        );
        assert!(
            !err.contains("not yet implemented") && !err.contains("would be generated"),
            "the vision stub/placeholder must be gone, got: {err}"
        );
    }

    /// Companion regression for `describe_video` — when the feature is enabled
    /// it must fail fast with an honest "not implemented" error (no fake
    /// success), and it must NOT gate on a Gemini key first (a key wouldn't
    /// help, so demanding one would just send the user on a wild goose chase).
    #[tokio::test]
    async fn describe_video_returns_not_implemented_error_when_enabled() {
        let engine = MediaEngine::new(MediaConfig {
            video_description: true,
            ..Default::default()
        });
        let attachment = MediaAttachment {
            media_type: MediaType::Video,
            mime_type: "video/mp4".into(),
            source: MediaSource::FilePath {
                path: "fixture.mp4".into(),
            },
            size_bytes: 1024,
        };
        let result = engine.describe_video(&attachment).await;
        assert!(
            result.is_err(),
            "describe_video must error, not return a fake stub"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("not yet implemented"),
            "error must clearly say video is unimplemented, got: {err}"
        );
        // No false-hope key requirement, and no leaked old placeholder.
        assert!(
            !err.contains("requires GEMINI_API_KEY"),
            "must not imply a key would help, got: {err}"
        );
        assert!(
            !err.contains("would be generated"),
            "must not leak the old placeholder phrase, got: {err}"
        );
    }
}
