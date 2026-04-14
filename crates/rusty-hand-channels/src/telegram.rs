//! Telegram Bot API adapter for the RustyHand channel bridge.
//!
//! Uses long-polling via `getUpdates` with exponential backoff on failures.
//! No external Telegram crate — just `reqwest` for full control over error handling.

use crate::types::{
    split_message, ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser,
};
use async_trait::async_trait;
use futures::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, warn};
use zeroize::Zeroizing;

/// Maximum backoff duration on API failures.
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// Initial backoff duration on API failures.
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
/// Telegram long-polling timeout (seconds) — sent as the `timeout` parameter to getUpdates.
const LONG_POLL_TIMEOUT: u64 = 30;

/// Telegram Bot API adapter using long-polling.
pub struct TelegramAdapter {
    /// SECURITY: Bot token is zeroized on drop to prevent memory disclosure.
    token: Zeroizing<String>,
    client: reqwest::Client,
    allowed_users: Vec<i64>,
    poll_interval: Duration,
    shutdown_tx: Arc<watch::Sender<bool>>,
    shutdown_rx: watch::Receiver<bool>,
}

impl TelegramAdapter {
    /// Create a new Telegram adapter.
    ///
    /// `token` is the raw bot token (read from env by the caller).
    /// `allowed_users` is the list of Telegram user IDs allowed to interact (empty = allow all).
    pub fn new(token: String, allowed_users: Vec<i64>, poll_interval: Duration) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            token: Zeroizing::new(token),
            client: reqwest::Client::new(),
            allowed_users,
            poll_interval,
            shutdown_tx: Arc::new(shutdown_tx),
            shutdown_rx,
        }
    }

    /// Validate the bot token by calling `getMe`.
    pub async fn validate_token(&self) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("https://api.telegram.org/bot{}/getMe", self.token.as_str());
        let resp: serde_json::Value = self.client.get(&url).send().await?.json().await?;

        if resp["ok"].as_bool() != Some(true) {
            let desc = resp["description"].as_str().unwrap_or("unknown error");
            return Err(format!("Telegram getMe failed: {desc}").into());
        }

        let bot_name = resp["result"]["username"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        Ok(bot_name)
    }

    /// Call `sendMessage` on the Telegram API.
    async fn api_send_message(
        &self,
        chat_id: i64,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.token.as_str()
        );

        // Telegram has a 4096 character limit per message — split if needed
        let chunks = split_message(text, 4096);
        for chunk in chunks {
            let body = serde_json::json!({
                "chat_id": chat_id,
                "text": chunk,
            });

            let resp = self.client.post(&url).json(&body).send().await?;
            let status = resp.status();
            if !status.is_success() {
                let body_text = resp.text().await.unwrap_or_default();
                warn!("Telegram sendMessage failed ({status}): {body_text}");
            }
        }
        Ok(())
    }

    /// Call `sendMessage` and return the message ID (for subsequent edits).
    async fn api_send_message_returning_id(
        &self,
        chat_id: i64,
        text: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.token.as_str()
        );
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
        });
        let resp: serde_json::Value = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;
        let msg_id = resp["result"]["message_id"]
            .as_i64()
            .ok_or("Missing message_id in sendMessage response")?;
        Ok(msg_id)
    }

    /// Call `editMessageText` to update an existing message (for streaming).
    async fn api_edit_message(
        &self,
        chat_id: i64,
        message_id: i64,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.telegram.org/bot{}/editMessageText",
            self.token.as_str()
        );
        let body = serde_json::json!({
            "chat_id": chat_id,
            "message_id": message_id,
            "text": text,
        });
        let resp = self.client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            // Ignore "message is not modified" errors (Telegram returns 400 for identical text)
            let body_text = resp.text().await.unwrap_or_default();
            if !body_text.contains("message is not modified") {
                warn!("Telegram editMessageText failed: {body_text}");
            }
        }
        Ok(())
    }

    /// Call `sendMessage` with an inline keyboard and return the message ID.
    async fn api_send_message_with_keyboard(
        &self,
        chat_id: i64,
        text: &str,
        buttons: &[Vec<crate::types::InlineButton>],
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.token.as_str()
        );
        let inline_keyboard: Vec<Vec<serde_json::Value>> = buttons
            .iter()
            .map(|row| {
                row.iter()
                    .map(|btn| {
                        serde_json::json!({
                            "text": btn.text,
                            "callback_data": btn.callback_data,
                        })
                    })
                    .collect()
            })
            .collect();
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "reply_markup": { "inline_keyboard": inline_keyboard },
        });
        let resp: serde_json::Value = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;
        let msg_id = resp["result"]["message_id"]
            .as_i64()
            .ok_or("Missing message_id in sendMessage response")?;
        Ok(msg_id)
    }

    /// Call `answerCallbackQuery` to dismiss the button spinner.
    async fn api_answer_callback_query(
        &self,
        callback_query_id: &str,
        text: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.telegram.org/bot{}/answerCallbackQuery",
            self.token.as_str()
        );
        let mut body = serde_json::json!({
            "callback_query_id": callback_query_id,
        });
        if let Some(t) = text {
            body["text"] = serde_json::json!(t);
        }
        let _ = self.client.post(&url).json(&body).send().await?;
        Ok(())
    }

    /// Call `editMessageReplyMarkup` to remove inline keyboard after user clicks.
    /// Used by the approval callback handler in bridge.rs.
    #[allow(dead_code)]
    async fn api_remove_keyboard(
        &self,
        chat_id: i64,
        message_id: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.telegram.org/bot{}/editMessageReplyMarkup",
            self.token.as_str()
        );
        let body = serde_json::json!({
            "chat_id": chat_id,
            "message_id": message_id,
            "reply_markup": { "inline_keyboard": [] },
        });
        let _ = self.client.post(&url).json(&body).send().await?;
        Ok(())
    }

    /// Send a file via `sendDocument` (multipart form-data).
    async fn api_send_document(
        &self,
        chat_id: i64,
        file_path: &str,
        caption: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendDocument",
            self.token.as_str()
        );
        let file_bytes = tokio::fs::read(file_path).await?;
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();
        let file_part = reqwest::multipart::Part::bytes(file_bytes).file_name(file_name);
        let mut form = reqwest::multipart::Form::new()
            .text("chat_id", chat_id.to_string())
            .part("document", file_part);
        if let Some(cap) = caption {
            form = form.text("caption", cap.to_string());
        }
        let resp = self.client.post(&url).multipart(form).send().await?;
        if !resp.status().is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            warn!("Telegram sendDocument failed: {body_text}");
        }
        Ok(())
    }

    /// Send a photo via `sendPhoto` — accepts file path or URL.
    async fn api_send_photo(
        &self,
        chat_id: i64,
        photo: &str,
        caption: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendPhoto",
            self.token.as_str()
        );
        // If it looks like a URL, send as string; otherwise read from disk
        if photo.starts_with("http://") || photo.starts_with("https://") {
            let mut body = serde_json::json!({
                "chat_id": chat_id,
                "photo": photo,
            });
            if let Some(cap) = caption {
                body["caption"] = serde_json::json!(cap);
            }
            let resp = self.client.post(&url).json(&body).send().await?;
            if !resp.status().is_success() {
                let body_text = resp.text().await.unwrap_or_default();
                warn!("Telegram sendPhoto (URL) failed: {body_text}");
            }
        } else {
            let file_bytes = tokio::fs::read(photo).await?;
            let file_name = std::path::Path::new(photo)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("photo.jpg")
                .to_string();
            let file_part = reqwest::multipart::Part::bytes(file_bytes).file_name(file_name);
            let mut form = reqwest::multipart::Form::new()
                .text("chat_id", chat_id.to_string())
                .part("photo", file_part);
            if let Some(cap) = caption {
                form = form.text("caption", cap.to_string());
            }
            let resp = self.client.post(&url).multipart(form).send().await?;
            if !resp.status().is_success() {
                let body_text = resp.text().await.unwrap_or_default();
                warn!("Telegram sendPhoto (file) failed: {body_text}");
            }
        }
        Ok(())
    }

    /// Send a voice message via `sendVoice` (multipart).
    async fn api_send_voice(
        &self,
        chat_id: i64,
        file_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendVoice",
            self.token.as_str()
        );
        let file_bytes = tokio::fs::read(file_path).await?;
        let file_part = reqwest::multipart::Part::bytes(file_bytes).file_name("voice.ogg");
        let form = reqwest::multipart::Form::new()
            .text("chat_id", chat_id.to_string())
            .part("voice", file_part);
        let resp = self.client.post(&url).multipart(form).send().await?;
        if !resp.status().is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            warn!("Telegram sendVoice failed: {body_text}");
        }
        Ok(())
    }

    /// Download a file from Telegram by file_id.
    ///
    /// Two-step process: `getFile` returns a file_path, then download from
    /// `https://api.telegram.org/file/bot{token}/{file_path}`.
    /// Saves to temp dir and returns the local path.
    async fn api_download_file(
        &self,
        file_id: &str,
        extension: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Step 1: getFile → get file_path
        let url = format!(
            "https://api.telegram.org/bot{}/getFile",
            self.token.as_str()
        );
        let body = serde_json::json!({"file_id": file_id});
        let resp: serde_json::Value = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if resp["ok"].as_bool() != Some(true) {
            let desc = resp["description"].as_str().unwrap_or("unknown error");
            return Err(format!("Telegram getFile failed: {desc}").into());
        }

        let file_path = resp["result"]["file_path"]
            .as_str()
            .ok_or("Missing file_path in getFile response")?;

        // Step 2: Download file content with size limit
        let download_url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            self.token.as_str(),
            file_path
        );
        const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50 MB
        let response = self.client.get(&download_url).send().await?;
        if let Some(len) = response.content_length() {
            if len > MAX_FILE_SIZE {
                return Err(format!("File too large: {len} bytes (max {MAX_FILE_SIZE})").into());
            }
        }
        let bytes = response.bytes().await?;

        // Save to temp directory (full UUID to prevent collisions)
        let dir = std::env::temp_dir().join("rusty_hand_telegram_media");
        tokio::fs::create_dir_all(&dir).await?;
        let local_path = dir.join(format!("{}.{extension}", uuid::Uuid::new_v4()));
        tokio::fs::write(&local_path, &bytes).await?;

        Ok(local_path.to_string_lossy().to_string())
    }

    /// Call `sendChatAction` to show "typing..." indicator.
    async fn api_send_typing(&self, chat_id: i64) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendChatAction",
            self.token.as_str()
        );
        let body = serde_json::json!({
            "chat_id": chat_id,
            "action": "typing",
        });
        let _ = self.client.post(&url).json(&body).send().await?;
        Ok(())
    }
}

#[async_trait]
impl ChannelAdapter for TelegramAdapter {
    fn name(&self) -> &str {
        "telegram"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Telegram
    }

    async fn start(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>, Box<dyn std::error::Error>>
    {
        // Validate token first (fail fast)
        let bot_name = self.validate_token().await?;
        info!("Telegram bot @{bot_name} connected");

        // SECURITY: warn if allowed_users is empty (bot accepts messages from anyone)
        if self.allowed_users.is_empty() {
            warn!(
                "Telegram bot @{bot_name} has no allowed_users configured — \
                 any Telegram user can interact with this bot. \
                 Set [channels.telegram].allowed_users in config to restrict access."
            );
        }

        let (tx, rx) = mpsc::channel::<ChannelMessage>(256);

        let token = self.token.clone();
        let client = self.client.clone();
        let allowed_users = self.allowed_users.clone();
        let poll_interval = self.poll_interval;
        let mut shutdown = self.shutdown_rx.clone();

        tokio::spawn(async move {
            let mut offset: Option<i64> = None;
            let mut backoff = INITIAL_BACKOFF;

            loop {
                // Check shutdown
                if *shutdown.borrow() {
                    break;
                }

                // Build getUpdates request
                let url = format!("https://api.telegram.org/bot{}/getUpdates", token.as_str());
                let mut params = serde_json::json!({
                    "timeout": LONG_POLL_TIMEOUT,
                    "allowed_updates": ["message", "edited_message", "callback_query"],
                });
                if let Some(off) = offset {
                    params["offset"] = serde_json::json!(off);
                }

                // Make the request with a timeout slightly longer than the long-poll timeout
                let request_timeout = Duration::from_secs(LONG_POLL_TIMEOUT + 10);
                let result = tokio::select! {
                    res = async {
                        client
                            .get(&url)
                            .json(&params)
                            .timeout(request_timeout)
                            .send()
                            .await
                    } => res,
                    _ = shutdown.changed() => {
                        break;
                    }
                };

                let resp = match result {
                    Ok(resp) => resp,
                    Err(e) => {
                        warn!("Telegram getUpdates network error: {e}, retrying in {backoff:?}");
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue;
                    }
                };

                let status = resp.status();

                // Handle rate limiting
                if status.as_u16() == 429 {
                    let body: serde_json::Value = resp.json().await.unwrap_or_default();
                    let retry_after = body["parameters"]["retry_after"].as_u64().unwrap_or(5);
                    warn!("Telegram rate limited, retry after {retry_after}s");
                    tokio::time::sleep(Duration::from_secs(retry_after)).await;
                    continue;
                }

                // Handle conflict (another bot instance polling)
                if status.as_u16() == 409 {
                    error!("Telegram 409 Conflict — another bot instance is running. Stopping.");
                    break;
                }

                if !status.is_success() {
                    let body_text = resp.text().await.unwrap_or_default();
                    warn!("Telegram getUpdates failed ({status}): {body_text}, retrying in {backoff:?}");
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                    continue;
                }

                // Parse response
                let body: serde_json::Value = match resp.json().await {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("Telegram getUpdates parse error: {e}");
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue;
                    }
                };

                // Reset backoff on success
                backoff = INITIAL_BACKOFF;

                if body["ok"].as_bool() != Some(true) {
                    warn!("Telegram getUpdates returned ok=false");
                    tokio::time::sleep(poll_interval).await;
                    continue;
                }

                let updates = match body["result"].as_array() {
                    Some(arr) => arr,
                    None => {
                        tokio::time::sleep(poll_interval).await;
                        continue;
                    }
                };

                for update in updates {
                    // Track offset for dedup
                    if let Some(update_id) = update["update_id"].as_i64() {
                        offset = Some(update_id + 1);
                    }

                    // Parse the message
                    let msg = match parse_telegram_update(update, &allowed_users) {
                        Some(m) => m,
                        None => continue, // filtered out or unparseable
                    };

                    debug!(
                        "Telegram message from chat_id={}: {:?}",
                        msg.sender.platform_id, msg.content
                    );

                    if tx.send(msg).await.is_err() {
                        // Receiver dropped — bridge is shutting down
                        return;
                    }
                }

                // Small delay between polls even on success to avoid tight loops
                tokio::time::sleep(poll_interval).await;
            }

            info!("Telegram polling loop stopped");
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn send(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let chat_id: i64 = user
            .platform_id
            .parse()
            .map_err(|_| format!("Invalid Telegram chat_id: {}", user.platform_id))?;

        match content {
            ChannelContent::Text(text) => {
                self.api_send_message(chat_id, &text).await?;
            }
            ChannelContent::Image { url, caption } => {
                self.api_send_photo(chat_id, &url, caption.as_deref())
                    .await?;
            }
            ChannelContent::File { url, filename } => {
                self.api_send_document(chat_id, &url, Some(&filename))
                    .await?;
            }
            ChannelContent::Voice { url, .. } => {
                self.api_send_voice(chat_id, &url).await?;
            }
            _ => {
                self.api_send_message(chat_id, "(Unsupported content type)")
                    .await?;
            }
        }
        Ok(())
    }

    async fn send_typing(&self, user: &ChannelUser) -> Result<(), Box<dyn std::error::Error>> {
        let chat_id: i64 = user
            .platform_id
            .parse()
            .map_err(|_| format!("Invalid Telegram chat_id: {}", user.platform_id))?;
        self.api_send_typing(chat_id).await
    }

    async fn send_with_buttons(
        &self,
        user: &ChannelUser,
        text: &str,
        buttons: &[Vec<crate::types::InlineButton>],
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let chat_id: i64 = user
            .platform_id
            .parse()
            .map_err(|_| format!("Invalid Telegram chat_id: {}", user.platform_id))?;
        let msg_id = self
            .api_send_message_with_keyboard(chat_id, text, buttons)
            .await?;
        Ok(Some(msg_id.to_string()))
    }

    async fn answer_callback(
        &self,
        callback_id: &str,
        text: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.api_answer_callback_query(callback_id, text).await
    }

    async fn download_file(
        &self,
        file_id: &str,
        extension: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.api_download_file(file_id, extension).await
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn send_streaming(
        &self,
        user: &ChannelUser,
        mut rx: tokio::sync::mpsc::Receiver<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let chat_id: i64 = user
            .platform_id
            .parse()
            .map_err(|_| format!("Invalid Telegram chat_id: {}", user.platform_id))?;

        let mut full_text = String::new();
        let mut message_id: Option<i64> = None;
        let mut last_edit = std::time::Instant::now();

        // Minimum interval between edits to avoid Telegram rate limits (429)
        const EDIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

        while let Some(chunk) = rx.recv().await {
            full_text.push_str(&chunk);

            // Send initial message or edit existing one
            if message_id.is_none() {
                // Send first chunk as new message
                if !full_text.is_empty() {
                    match self
                        .api_send_message_returning_id(chat_id, &full_text)
                        .await
                    {
                        Ok(mid) => message_id = Some(mid),
                        Err(e) => {
                            warn!("Telegram streaming: failed to send initial message: {e}");
                            break;
                        }
                    }
                    last_edit = std::time::Instant::now();
                }
            } else if last_edit.elapsed() >= EDIT_INTERVAL {
                // Edit message with accumulated text (throttled)
                if let Some(mid) = message_id {
                    // Truncate to Telegram's 4096 char limit
                    let display: String = full_text.chars().take(4096).collect();
                    let _ = self.api_edit_message(chat_id, mid, &display).await;
                    last_edit = std::time::Instant::now();
                }
            }
        }

        // Final edit with complete text
        if let Some(mid) = message_id {
            if !full_text.is_empty() {
                // If text exceeds 4096, send overflow as new messages
                let chunks = crate::types::split_message(&full_text, 4096);
                let first = &chunks[0];
                if chunks.len() == 1 {
                    let _ = self.api_edit_message(chat_id, mid, first).await;
                } else {
                    // Edit first message, send rest as new messages
                    let _ = self.api_edit_message(chat_id, mid, first).await;
                    for extra in &chunks[1..] {
                        let _ = self.api_send_message(chat_id, extra).await;
                    }
                }
            }
        } else if !full_text.is_empty() {
            // Fallback: no initial message was sent, send full text
            self.api_send_message(chat_id, &full_text).await?;
        }

        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.shutdown_tx.send(true);
        Ok(())
    }
}

/// Parse a Telegram update JSON into a `ChannelMessage`, or `None` if filtered/unparseable.
/// Handles `message`, `edited_message`, and `callback_query` update types.
fn parse_telegram_update(
    update: &serde_json::Value,
    allowed_users: &[i64],
) -> Option<ChannelMessage> {
    // Handle callback_query (inline keyboard button press)
    if let Some(callback) = update.get("callback_query") {
        let from = callback.get("from")?;
        let user_id = from["id"].as_i64()?;
        if !allowed_users.is_empty() && !allowed_users.contains(&user_id) {
            return None;
        }
        let chat_id = callback["message"]["chat"]["id"].as_i64()?;
        let callback_id = callback["id"].as_str()?.to_string();
        let data = callback["data"].as_str().unwrap_or("").to_string();
        let first_name = from["first_name"].as_str().unwrap_or("Unknown");
        return Some(ChannelMessage {
            channel: ChannelType::Telegram,
            platform_message_id: callback["message"]["message_id"]
                .as_i64()
                .unwrap_or(0)
                .to_string(),
            sender: ChannelUser {
                platform_id: chat_id.to_string(),
                display_name: first_name.to_string(),
                rusty_hand_user: None,
            },
            content: ChannelContent::CallbackQuery { data, callback_id },
            target_agent: None,
            timestamp: chrono::Utc::now(),
            is_group: false,
            thread_id: None,
            metadata: HashMap::new(),
        });
    }

    let message = update
        .get("message")
        .or_else(|| update.get("edited_message"))?;
    let from = message.get("from")?;
    let user_id = from["id"].as_i64()?;

    // Security: check allowed_users
    if !allowed_users.is_empty() && !allowed_users.contains(&user_id) {
        debug!("Telegram: ignoring message from unlisted user {user_id}");
        return None;
    }

    let chat_id = message["chat"]["id"].as_i64()?;
    let first_name = from["first_name"].as_str().unwrap_or("Unknown");
    let last_name = from["last_name"].as_str().unwrap_or("");
    let display_name = if last_name.is_empty() {
        first_name.to_string()
    } else {
        format!("{first_name} {last_name}")
    };

    let chat_type = message["chat"]["type"].as_str().unwrap_or("private");
    let is_group = chat_type == "group" || chat_type == "supergroup";

    let message_id = message["message_id"].as_i64().unwrap_or(0);
    let timestamp = message["date"]
        .as_i64()
        .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
        .unwrap_or_else(chrono::Utc::now);
    let caption = message["caption"].as_str().map(String::from);

    // Determine content: check media types first, then text.
    // Media messages store file_id in metadata for deferred download by the bridge.
    let (content, mut metadata) = if let Some(photos) = message["photo"].as_array() {
        // Photo: take the highest-resolution variant (last in array)
        let best = photos.last()?;
        let file_id = best["file_id"].as_str()?.to_string();
        let mut meta = HashMap::new();
        meta.insert(
            "file_id".to_string(),
            serde_json::Value::String(file_id.clone()),
        );
        meta.insert(
            "media_type".to_string(),
            serde_json::Value::String("photo".to_string()),
        );
        (
            ChannelContent::Image {
                url: file_id,
                caption: caption.clone(),
            },
            meta,
        )
    } else if let Some(voice) = message.get("voice") {
        let file_id = voice["file_id"].as_str()?.to_string();
        let duration = voice["duration"].as_u64().unwrap_or(0) as u32;
        let mut meta = HashMap::new();
        meta.insert(
            "file_id".to_string(),
            serde_json::Value::String(file_id.clone()),
        );
        meta.insert(
            "media_type".to_string(),
            serde_json::Value::String("voice".to_string()),
        );
        (
            ChannelContent::Voice {
                url: file_id,
                duration_seconds: duration,
            },
            meta,
        )
    } else if let Some(audio) = message.get("audio") {
        let file_id = audio["file_id"].as_str()?.to_string();
        let duration = audio["duration"].as_u64().unwrap_or(0) as u32;
        let mut meta = HashMap::new();
        meta.insert(
            "file_id".to_string(),
            serde_json::Value::String(file_id.clone()),
        );
        meta.insert(
            "media_type".to_string(),
            serde_json::Value::String("audio".to_string()),
        );
        (
            ChannelContent::Voice {
                url: file_id,
                duration_seconds: duration,
            },
            meta,
        )
    } else if let Some(document) = message.get("document") {
        let file_id = document["file_id"].as_str()?.to_string();
        let file_name = document["file_name"].as_str().unwrap_or("file").to_string();
        let mut meta = HashMap::new();
        meta.insert(
            "file_id".to_string(),
            serde_json::Value::String(file_id.clone()),
        );
        meta.insert(
            "media_type".to_string(),
            serde_json::Value::String("document".to_string()),
        );
        (
            ChannelContent::File {
                url: file_id,
                filename: file_name,
            },
            meta,
        )
    } else if let Some(text) = message["text"].as_str() {
        // Text message — check for bot commands
        let content = if let Some(entities) = message["entities"].as_array() {
            let is_bot_command = entities.iter().any(|e| {
                e["type"].as_str() == Some("bot_command") && e["offset"].as_i64() == Some(0)
            });
            if is_bot_command {
                let parts: Vec<&str> = text.splitn(2, ' ').collect();
                let cmd_name = parts[0].trim_start_matches('/');
                let cmd_name = cmd_name.split('@').next().unwrap_or(cmd_name);
                let args = if parts.len() > 1 {
                    parts[1].split_whitespace().map(String::from).collect()
                } else {
                    vec![]
                };
                ChannelContent::Command {
                    name: cmd_name.to_string(),
                    args,
                }
            } else {
                ChannelContent::Text(text.to_string())
            }
        } else {
            ChannelContent::Text(text.to_string())
        };
        (content, HashMap::new())
    } else {
        // Unsupported message type (sticker, contact, location, etc.)
        return None;
    };

    // Add caption as metadata if present (for media with captions)
    if let Some(ref cap) = caption {
        metadata.insert(
            "caption".to_string(),
            serde_json::Value::String(cap.clone()),
        );
    }

    // Use chat_id as the platform_id (so responses go to the right chat)
    Some(ChannelMessage {
        channel: ChannelType::Telegram,
        platform_message_id: message_id.to_string(),
        sender: ChannelUser {
            platform_id: chat_id.to_string(),
            display_name,
            rusty_hand_user: None,
        },
        content,
        target_agent: None,
        timestamp,
        is_group,
        thread_id: None,
        metadata,
    })
}

/// Calculate exponential backoff capped at MAX_BACKOFF.
pub fn calculate_backoff(current: Duration) -> Duration {
    (current * 2).min(MAX_BACKOFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_telegram_update() {
        let update = serde_json::json!({
            "update_id": 123456,
            "message": {
                "message_id": 42,
                "from": {
                    "id": 111222333,
                    "first_name": "Alice",
                    "last_name": "Smith"
                },
                "chat": {
                    "id": 111222333,
                    "type": "private"
                },
                "date": 1700000000,
                "text": "Hello, agent!"
            }
        });

        let msg = parse_telegram_update(&update, &[]).unwrap();
        assert_eq!(msg.channel, ChannelType::Telegram);
        assert_eq!(msg.sender.display_name, "Alice Smith");
        assert_eq!(msg.sender.platform_id, "111222333");
        assert!(matches!(msg.content, ChannelContent::Text(ref t) if t == "Hello, agent!"));
    }

    #[test]
    fn test_parse_telegram_command() {
        let update = serde_json::json!({
            "update_id": 123457,
            "message": {
                "message_id": 43,
                "from": {
                    "id": 111222333,
                    "first_name": "Alice"
                },
                "chat": {
                    "id": 111222333,
                    "type": "private"
                },
                "date": 1700000001,
                "text": "/agent hello-world",
                "entities": [{
                    "type": "bot_command",
                    "offset": 0,
                    "length": 6
                }]
            }
        });

        let msg = parse_telegram_update(&update, &[]).unwrap();
        match &msg.content {
            ChannelContent::Command { name, args } => {
                assert_eq!(name, "agent");
                assert_eq!(args, &["hello-world"]);
            }
            other => unreachable!("Expected Command, got {other:?}"),
        }
    }

    #[test]
    fn test_allowed_users_filter() {
        let update = serde_json::json!({
            "update_id": 123458,
            "message": {
                "message_id": 44,
                "from": {
                    "id": 999,
                    "first_name": "Bob"
                },
                "chat": {
                    "id": 999,
                    "type": "private"
                },
                "date": 1700000002,
                "text": "blocked"
            }
        });

        // Empty allowed_users = allow all
        let msg = parse_telegram_update(&update, &[]);
        assert!(msg.is_some());

        // Non-matching allowed_users = filter out
        let msg = parse_telegram_update(&update, &[111, 222]);
        assert!(msg.is_none());

        // Matching allowed_users = allow
        let msg = parse_telegram_update(&update, &[999]);
        assert!(msg.is_some());
    }

    #[test]
    fn test_parse_telegram_edited_message() {
        let update = serde_json::json!({
            "update_id": 123459,
            "edited_message": {
                "message_id": 42,
                "from": {
                    "id": 111222333,
                    "first_name": "Alice",
                    "last_name": "Smith"
                },
                "chat": {
                    "id": 111222333,
                    "type": "private"
                },
                "date": 1700000000,
                "edit_date": 1700000060,
                "text": "Edited message!"
            }
        });

        let msg = parse_telegram_update(&update, &[]).unwrap();
        assert_eq!(msg.channel, ChannelType::Telegram);
        assert_eq!(msg.sender.display_name, "Alice Smith");
        assert!(matches!(msg.content, ChannelContent::Text(ref t) if t == "Edited message!"));
    }

    #[test]
    fn test_backoff_calculation() {
        let b1 = calculate_backoff(Duration::from_secs(1));
        assert_eq!(b1, Duration::from_secs(2));

        let b2 = calculate_backoff(Duration::from_secs(2));
        assert_eq!(b2, Duration::from_secs(4));

        let b3 = calculate_backoff(Duration::from_secs(32));
        assert_eq!(b3, Duration::from_secs(60)); // capped

        let b4 = calculate_backoff(Duration::from_secs(60));
        assert_eq!(b4, Duration::from_secs(60)); // stays at cap
    }

    #[test]
    fn test_parse_command_with_botname() {
        let update = serde_json::json!({
            "update_id": 100,
            "message": {
                "message_id": 1,
                "from": { "id": 123, "first_name": "X" },
                "chat": { "id": 123, "type": "private" },
                "date": 1700000000,
                "text": "/agents@myrustyhandbot",
                "entities": [{ "type": "bot_command", "offset": 0, "length": 17 }]
            }
        });

        let msg = parse_telegram_update(&update, &[]).unwrap();
        match &msg.content {
            ChannelContent::Command { name, args } => {
                assert_eq!(name, "agents");
                assert!(args.is_empty());
            }
            other => unreachable!("Expected Command, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_callback_query() {
        let update = serde_json::json!({
            "update_id": 999,
            "callback_query": {
                "id": "cb_12345",
                "from": { "id": 111, "first_name": "User", "is_bot": false },
                "message": {
                    "message_id": 42,
                    "chat": { "id": 111, "type": "private" }
                },
                "data": "approve:abc12345"
            }
        });
        let msg = parse_telegram_update(&update, &[]).unwrap();
        assert_eq!(msg.sender.platform_id, "111");
        match &msg.content {
            ChannelContent::CallbackQuery { data, callback_id } => {
                assert_eq!(data, "approve:abc12345");
                assert_eq!(callback_id, "cb_12345");
            }
            other => unreachable!("Expected CallbackQuery, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_callback_query_filtered_by_allowed_users() {
        let update = serde_json::json!({
            "update_id": 1000,
            "callback_query": {
                "id": "cb_999",
                "from": { "id": 999, "first_name": "Stranger" },
                "message": {
                    "message_id": 42,
                    "chat": { "id": 999, "type": "private" }
                },
                "data": "approve:x"
            }
        });
        // Stranger not in allowed_users → filtered out
        assert!(parse_telegram_update(&update, &[111]).is_none());
    }

    #[test]
    fn test_parse_callback_query_null_message() {
        // Telegram can send callback_query with null message (if original was deleted)
        let update = serde_json::json!({
            "update_id": 1001,
            "callback_query": {
                "id": "cb_888",
                "from": { "id": 111, "first_name": "User" },
                "data": "reject:xyz"
            }
        });
        // Missing message → None (graceful drop)
        assert!(parse_telegram_update(&update, &[]).is_none());
    }

    #[test]
    fn test_parse_photo_with_caption() {
        let update = serde_json::json!({
            "update_id": 123460,
            "message": {
                "message_id": 50,
                "from": { "id": 111, "first_name": "Alice" },
                "chat": { "id": 111, "type": "private" },
                "date": 1700000010,
                "photo": [
                    { "file_id": "small_id", "width": 90, "height": 90 },
                    { "file_id": "large_id", "width": 800, "height": 600 }
                ],
                "caption": "Look at this"
            }
        });
        let msg = parse_telegram_update(&update, &[]).unwrap();
        // Should take highest resolution (last element)
        match &msg.content {
            ChannelContent::Image { url, caption } => {
                assert_eq!(url, "large_id");
                assert_eq!(caption, &Some("Look at this".to_string()));
            }
            other => unreachable!("Expected Image, got {other:?}"),
        }
        // Caption also in metadata
        assert_eq!(
            msg.metadata.get("caption").and_then(|v| v.as_str()),
            Some("Look at this")
        );
        assert_eq!(
            msg.metadata.get("media_type").and_then(|v| v.as_str()),
            Some("photo")
        );
    }

    #[test]
    fn test_parse_voice_message() {
        let update = serde_json::json!({
            "update_id": 123461,
            "message": {
                "message_id": 51,
                "from": { "id": 111, "first_name": "Alice" },
                "chat": { "id": 111, "type": "private" },
                "date": 1700000011,
                "voice": {
                    "file_id": "voice_file_id",
                    "duration": 5,
                    "mime_type": "audio/ogg"
                }
            }
        });
        let msg = parse_telegram_update(&update, &[]).unwrap();
        match &msg.content {
            ChannelContent::Voice {
                url,
                duration_seconds,
            } => {
                assert_eq!(url, "voice_file_id");
                assert_eq!(*duration_seconds, 5);
            }
            other => unreachable!("Expected Voice, got {other:?}"),
        }
        assert_eq!(
            msg.metadata.get("media_type").and_then(|v| v.as_str()),
            Some("voice")
        );
    }

    #[test]
    fn test_parse_document_message() {
        let update = serde_json::json!({
            "update_id": 123462,
            "message": {
                "message_id": 52,
                "from": { "id": 111, "first_name": "Alice" },
                "chat": { "id": 111, "type": "private" },
                "date": 1700000012,
                "document": {
                    "file_id": "doc_file_id",
                    "file_name": "report.pdf",
                    "mime_type": "application/pdf"
                }
            }
        });
        let msg = parse_telegram_update(&update, &[]).unwrap();
        match &msg.content {
            ChannelContent::File { url, filename } => {
                assert_eq!(url, "doc_file_id");
                assert_eq!(filename, "report.pdf");
            }
            other => unreachable!("Expected File, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_unsupported_message_type() {
        // Sticker: no text, no photo, no voice, no document → None
        let update = serde_json::json!({
            "update_id": 123463,
            "message": {
                "message_id": 53,
                "from": { "id": 111, "first_name": "Alice" },
                "chat": { "id": 111, "type": "private" },
                "date": 1700000013,
                "sticker": { "file_id": "sticker_id" }
            }
        });
        assert!(parse_telegram_update(&update, &[]).is_none());
    }
}
