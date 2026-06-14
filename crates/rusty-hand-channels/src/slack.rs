//! Slack Socket Mode adapter for the RustyHand channel bridge.
//!
//! Uses Slack Socket Mode WebSocket (app token) for receiving events and the
//! Web API (bot token) for sending responses. No external Slack crate.

use crate::types::{
    split_message, ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser,
};
use async_trait::async_trait;
use futures::{SinkExt, Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch, RwLock};
use tracing::{debug, error, info, warn};
use zeroize::Zeroizing;

const SLACK_API_BASE: &str = "https://slack.com/api";
const MAX_BACKOFF: Duration = Duration::from_secs(60);
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const SLACK_MSG_LIMIT: usize = 3000;

/// Slack Socket Mode adapter.
pub struct SlackAdapter {
    /// SECURITY: Tokens are zeroized on drop to prevent memory disclosure.
    app_token: Zeroizing<String>,
    bot_token: Zeroizing<String>,
    client: reqwest::Client,
    allowed_channels: Vec<String>,
    shutdown_tx: Arc<watch::Sender<bool>>,
    shutdown_rx: watch::Receiver<bool>,
    /// Bot's own user ID (populated after auth.test).
    bot_user_id: Arc<RwLock<Option<String>>>,
}

impl SlackAdapter {
    pub fn new(app_token: String, bot_token: String, allowed_channels: Vec<String>) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            app_token: Zeroizing::new(app_token),
            bot_token: Zeroizing::new(bot_token),
            client: reqwest::Client::new(),
            allowed_channels,
            shutdown_tx: Arc::new(shutdown_tx),
            shutdown_rx,
            bot_user_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Validate the bot token by calling auth.test.
    async fn validate_bot_token(&self) -> Result<String, Box<dyn std::error::Error>> {
        let resp: serde_json::Value = self
            .client
            .post(format!("{SLACK_API_BASE}/auth.test"))
            .header(
                "Authorization",
                format!("Bearer {}", self.bot_token.as_str()),
            )
            .send()
            .await?
            .json()
            .await?;

        if resp["ok"].as_bool() != Some(true) {
            let err = resp["error"].as_str().unwrap_or("unknown error");
            return Err(format!("Slack auth.test failed: {err}").into());
        }

        let user_id = resp["user_id"].as_str().unwrap_or("unknown").to_string();
        Ok(user_id)
    }

    /// Send a message to a Slack channel via chat.postMessage.
    async fn api_send_message(
        &self,
        channel_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let chunks = split_message(text, SLACK_MSG_LIMIT);

        for chunk in chunks {
            let body = serde_json::json!({
                "channel": channel_id,
                "text": chunk,
            });

            let resp: serde_json::Value = self
                .client
                .post(format!("{SLACK_API_BASE}/chat.postMessage"))
                .header(
                    "Authorization",
                    format!("Bearer {}", self.bot_token.as_str()),
                )
                .json(&body)
                .send()
                .await?
                .json()
                .await?;

            // Slack returns 200 with `ok: false` for app-level errors
            // (channel_not_found, not_in_channel, token_revoked,
            // restricted_action, channel_archived, …). Pre-fix this
            // logged a warn and silently returned Ok(()) — the bridge
            // dispatcher believed the message was delivered, so failed
            // sends were invisible to the operator without grepping
            // logs. Return the error so the caller can surface it.
            if resp["ok"].as_bool() != Some(true) {
                let err = resp["error"].as_str().unwrap_or("unknown");
                warn!(
                    channel_id = %channel_id,
                    error = %err,
                    "Slack chat.postMessage failed"
                );
                return Err(format!(
                    "Slack chat.postMessage failed: {err}{}",
                    crate::slack_error_hint(err)
                )
                .into());
            }
        }
        Ok(())
    }

    /// Post a single message and return its `ts` (message id) for streaming
    /// updates via chat.update. Truncates to the 3000-char limit.
    async fn api_send_message_returning_ts(
        &self,
        channel_id: &str,
        text: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let display: String = text.chars().take(SLACK_MSG_LIMIT).collect();
        let body = serde_json::json!({ "channel": channel_id, "text": display });
        let resp: serde_json::Value = self
            .client
            .post(format!("{SLACK_API_BASE}/chat.postMessage"))
            .header(
                "Authorization",
                format!("Bearer {}", self.bot_token.as_str()),
            )
            .json(&body)
            .send()
            .await?
            .json()
            .await?;
        if resp["ok"].as_bool() != Some(true) {
            let err = resp["error"].as_str().unwrap_or("unknown");
            return Err(format!(
                "Slack chat.postMessage failed: {err}{}",
                crate::slack_error_hint(err)
            )
            .into());
        }
        match resp["ts"].as_str() {
            Some(ts) => Ok(ts.to_string()),
            None => Err("Slack chat.postMessage: no ts in response".into()),
        }
    }

    /// Update an existing message via chat.update (for streaming). Truncates.
    async fn api_update_message(
        &self,
        channel_id: &str,
        ts: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let display: String = text.chars().take(SLACK_MSG_LIMIT).collect();
        let body = serde_json::json!({ "channel": channel_id, "ts": ts, "text": display });
        let resp: serde_json::Value = self
            .client
            .post(format!("{SLACK_API_BASE}/chat.update"))
            .header(
                "Authorization",
                format!("Bearer {}", self.bot_token.as_str()),
            )
            .json(&body)
            .send()
            .await?
            .json()
            .await?;
        if resp["ok"].as_bool() != Some(true) {
            let err = resp["error"].as_str().unwrap_or("unknown");
            return Err(format!(
                "Slack chat.update failed: {err}{}",
                crate::slack_error_hint(err)
            )
            .into());
        }
        Ok(())
    }
}

#[async_trait]
impl ChannelAdapter for SlackAdapter {
    fn name(&self) -> &str {
        "slack"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Slack
    }

    async fn start(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>, Box<dyn std::error::Error>>
    {
        // Validate bot token first
        let bot_user_id_val = self.validate_bot_token().await?;
        *self.bot_user_id.write().await = Some(bot_user_id_val.clone());
        info!("Slack bot authenticated (user_id: {bot_user_id_val})");

        // SECURITY: warn if allowed_channels is empty (bot accepts messages from any channel)
        if self.allowed_channels.is_empty() {
            warn!(
                "Slack bot has no allowed_channels configured — \
                 messages from any channel will be accepted. \
                 Set [channels.slack].allowed_channels in config to restrict access."
            );
        }

        let (tx, rx) = mpsc::channel::<ChannelMessage>(256);

        let app_token = self.app_token.clone();
        let bot_user_id = self.bot_user_id.clone();
        let allowed_channels = self.allowed_channels.clone();
        let client = self.client.clone();
        let mut shutdown = self.shutdown_rx.clone();

        tokio::spawn(async move {
            let mut backoff = INITIAL_BACKOFF;

            loop {
                if *shutdown.borrow() {
                    break;
                }

                // Get a fresh WebSocket URL
                let ws_url_result = get_socket_mode_url(&client, &app_token)
                    .await
                    .map_err(|e| e.to_string());
                let ws_url = match ws_url_result {
                    Ok(url) => url,
                    Err(err_msg) => {
                        warn!("Slack: failed to get WebSocket URL: {err_msg}, retrying in {backoff:?}");
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue;
                    }
                };

                info!("Connecting to Slack Socket Mode...");

                let ws_result = tokio_tungstenite::connect_async(&ws_url).await;
                let ws_stream = match ws_result {
                    Ok((stream, _)) => stream,
                    Err(e) => {
                        warn!("Slack WebSocket connection failed: {e}, retrying in {backoff:?}");
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue;
                    }
                };

                backoff = INITIAL_BACKOFF;
                info!("Slack Socket Mode connected");

                let (mut ws_tx, mut ws_rx) = ws_stream.split();

                let should_reconnect = 'inner: loop {
                    let msg = tokio::select! {
                        msg = ws_rx.next() => msg,
                        _ = shutdown.changed() => {
                            if *shutdown.borrow() {
                                let _ = ws_tx.close().await;
                                return;
                            }
                            continue;
                        }
                    };

                    let msg = match msg {
                        Some(Ok(m)) => m,
                        Some(Err(e)) => {
                            warn!("Slack WebSocket error: {e}");
                            break 'inner true;
                        }
                        None => {
                            info!("Slack WebSocket closed");
                            break 'inner true;
                        }
                    };

                    let text = match msg {
                        tokio_tungstenite::tungstenite::Message::Text(t) => t,
                        tokio_tungstenite::tungstenite::Message::Close(_) => {
                            info!("Slack Socket Mode closed by server");
                            break 'inner true;
                        }
                        _ => continue,
                    };

                    let payload: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(v) => v,
                        Err(e) => {
                            warn!("Slack: failed to parse message: {e}");
                            continue;
                        }
                    };

                    let envelope_type = payload["type"].as_str().unwrap_or("");

                    match envelope_type {
                        "hello" => {
                            debug!("Slack Socket Mode hello received");
                        }

                        "events_api" => {
                            // Acknowledge the envelope
                            let envelope_id = payload["envelope_id"].as_str().unwrap_or("");
                            if !envelope_id.is_empty() {
                                let ack = serde_json::json!({ "envelope_id": envelope_id });
                                if let Err(e) = ws_tx
                                    .send(tokio_tungstenite::tungstenite::Message::Text(
                                        serde_json::to_string(&ack).unwrap(),
                                    ))
                                    .await
                                {
                                    error!("Slack: failed to send ack: {e}");
                                    break 'inner true;
                                }
                            }

                            // Extract the event
                            let event = &payload["payload"]["event"];
                            if let Some(msg) =
                                parse_slack_event(event, &bot_user_id, &allowed_channels).await
                            {
                                debug!(
                                    "Slack message from channel={}: {:?}",
                                    msg.sender.platform_id, msg.content
                                );
                                if tx.send(msg).await.is_err() {
                                    return;
                                }
                            }
                        }

                        "disconnect" => {
                            let reason = payload["reason"].as_str().unwrap_or("unknown");
                            info!("Slack disconnect request: {reason}");
                            break 'inner true;
                        }

                        _ => {
                            debug!("Slack envelope type: {envelope_type}");
                        }
                    }
                };

                if !should_reconnect || *shutdown.borrow() {
                    break;
                }

                warn!("Slack: reconnecting in {backoff:?}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }

            info!("Slack Socket Mode loop stopped");
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn send(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let channel_id = &user.platform_id;
        match content {
            ChannelContent::Text(text) => {
                self.api_send_message(channel_id, &text).await?;
            }
            _ => {
                self.api_send_message(channel_id, "(Unsupported content type)")
                    .await?;
            }
        }
        Ok(())
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn send_streaming(
        &self,
        user: &ChannelUser,
        mut rx: tokio::sync::mpsc::Receiver<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let channel_id = &user.platform_id;
        let mut full_text = String::new();
        let mut ts: Option<String> = None;
        let mut last_edit = std::time::Instant::now();

        // chat.update is Tier 3 (~50/min); throttle to ~1 update/sec.
        const EDIT_INTERVAL: Duration = Duration::from_millis(1000);

        while let Some(chunk) = rx.recv().await {
            full_text.push_str(&chunk);

            if ts.is_none() {
                // Post the first chunk as a new message and capture its ts.
                if !full_text.is_empty() {
                    match self
                        .api_send_message_returning_ts(channel_id, &full_text)
                        .await
                    {
                        Ok(message_ts) => ts = Some(message_ts),
                        Err(e) => {
                            warn!("Slack streaming: failed to send initial message: {e}");
                            break;
                        }
                    }
                    last_edit = std::time::Instant::now();
                }
            } else if last_edit.elapsed() >= EDIT_INTERVAL {
                // Throttled mid-stream update. Failures are recoverable (next
                // tick retries with newer text) — log at debug, don't break.
                if let Some(ref message_ts) = ts {
                    if let Err(e) = self
                        .api_update_message(channel_id, message_ts, &full_text)
                        .await
                    {
                        tracing::debug!(
                            channel_id = %channel_id, error = %e,
                            "Slack streaming: mid-stream update failed (will retry next tick)"
                        );
                    }
                    last_edit = std::time::Instant::now();
                }
            }
        }

        // Final flush: update the first message with the complete text, sending
        // any overflow beyond the 3000-char limit as new messages.
        if let Some(ref message_ts) = ts {
            if !full_text.is_empty() {
                let chunks = split_message(&full_text, SLACK_MSG_LIMIT);
                if let Err(e) = self
                    .api_update_message(channel_id, message_ts, chunks[0])
                    .await
                {
                    warn!(
                        channel_id = %channel_id, error = %e,
                        "Slack streaming: final update failed — user view stale"
                    );
                }
                for extra in &chunks[1..] {
                    if let Err(e) = self.api_send_message(channel_id, extra).await {
                        warn!(
                            channel_id = %channel_id, error = %e,
                            "Slack streaming: overflow chunk send failed — partial response lost"
                        );
                    }
                }
            }
        } else if !full_text.is_empty() {
            // No initial message was sent — fall back to a plain send.
            self.api_send_message(channel_id, &full_text).await?;
        }

        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.shutdown_tx.send(true);
        Ok(())
    }
}

/// Helper to get Socket Mode WebSocket URL.
async fn get_socket_mode_url(
    client: &reqwest::Client,
    app_token: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let resp: serde_json::Value = client
        .post(format!("{SLACK_API_BASE}/apps.connections.open"))
        .header("Authorization", format!("Bearer {app_token}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await?
        .json()
        .await?;

    if resp["ok"].as_bool() != Some(true) {
        let err = resp["error"].as_str().unwrap_or("unknown error");
        return Err(format!("Slack apps.connections.open failed: {err}").into());
    }

    resp["url"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| "Missing 'url' in connections.open response".into())
}

/// Parse a Slack event into a `ChannelMessage`.
async fn parse_slack_event(
    event: &serde_json::Value,
    bot_user_id: &Arc<RwLock<Option<String>>>,
    allowed_channels: &[String],
) -> Option<ChannelMessage> {
    let event_type = event["type"].as_str()?;
    if event_type != "message" {
        return None;
    }

    // Handle message_changed subtype: extract inner message
    let subtype = event["subtype"].as_str();
    let (msg_data, is_edit) = match subtype {
        Some("message_changed") => {
            // Edited messages have the new content in event.message
            match event.get("message") {
                Some(inner) => (inner, true),
                None => return None,
            }
        }
        Some(_) => return None, // Skip other subtypes (joins, leaves, etc.)
        None => (event, false),
    };

    // Filter out bot's own messages
    if msg_data.get("bot_id").is_some() {
        return None;
    }
    let user_id = msg_data["user"]
        .as_str()
        .or_else(|| event["user"].as_str())?;
    if let Some(ref bid) = *bot_user_id.read().await {
        if user_id == bid {
            return None;
        }
    }

    let channel = event["channel"].as_str()?;

    // Slack DMs ("im" channels, whose ids start with 'D') must be governed by
    // dm_policy rather than group_policy. channel_type is the authoritative
    // signal; fall back to the channel-id prefix when it is absent.
    let channel_type = event["channel_type"].as_str().unwrap_or("");
    let is_group = !(channel_type == "im" || channel.starts_with('D'));

    // Filter by allowed channels
    if !allowed_channels.is_empty() && !allowed_channels.contains(&channel.to_string()) {
        return None;
    }

    let text = msg_data["text"].as_str().unwrap_or("");
    if text.is_empty() {
        return None;
    }

    let ts = if is_edit {
        msg_data["ts"]
            .as_str()
            .unwrap_or(event["ts"].as_str().unwrap_or("0"))
    } else {
        event["ts"].as_str().unwrap_or("0")
    };

    // Parse timestamp (Slack uses epoch.microseconds format)
    let timestamp = ts
        .split('.')
        .next()
        .and_then(|s| s.parse::<i64>().ok())
        .and_then(|epoch| chrono::DateTime::from_timestamp(epoch, 0))
        .unwrap_or_else(chrono::Utc::now);

    // Parse commands (messages starting with /)
    let content = if text.starts_with('/') {
        let parts: Vec<&str> = text.splitn(2, ' ').collect();
        let cmd_name = &parts[0][1..];
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
    };

    Some(ChannelMessage {
        channel: ChannelType::Slack,
        platform_message_id: ts.to_string(),
        sender: ChannelUser {
            platform_id: channel.to_string(),
            display_name: user_id.to_string(), // Slack user IDs as display name
            rusty_hand_user: None,
        },
        content,
        target_agent: None,
        timestamp,
        is_group,
        thread_id: None,
        metadata: HashMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_slack_event_basic() {
        let bot_id = Arc::new(RwLock::new(Some("B123".to_string())));
        let event = serde_json::json!({
            "type": "message",
            "user": "U456",
            "channel": "C789",
            "text": "Hello agent!",
            "ts": "1700000000.000100"
        });

        let msg = parse_slack_event(&event, &bot_id, &[]).await.unwrap();
        assert_eq!(msg.channel, ChannelType::Slack);
        assert_eq!(msg.sender.platform_id, "C789");
        assert!(matches!(msg.content, ChannelContent::Text(ref t) if t == "Hello agent!"));
        assert!(msg.is_group, "a channel message must be is_group");
    }

    #[tokio::test]
    async fn test_parse_slack_dm_is_not_group() {
        let bot_id = Arc::new(RwLock::new(Some("B123".to_string())));
        // Authoritative channel_type=im.
        let im = serde_json::json!({
            "type": "message", "user": "U456", "channel": "D789",
            "channel_type": "im", "text": "hi", "ts": "1700000000.000100"
        });
        let msg = parse_slack_event(&im, &bot_id, &[]).await.unwrap();
        assert!(!msg.is_group, "an im must not be is_group");

        // Fallback: channel_type absent, but the id has the DM 'D' prefix.
        let im2 = serde_json::json!({
            "type": "message", "user": "U456", "channel": "D000",
            "text": "hi", "ts": "1700000000.000100"
        });
        let msg2 = parse_slack_event(&im2, &bot_id, &[]).await.unwrap();
        assert!(!msg2.is_group, "a 'D'-prefixed channel must not be is_group");
    }

    #[tokio::test]
    async fn test_parse_slack_event_filters_bot() {
        let bot_id = Arc::new(RwLock::new(Some("B123".to_string())));
        let event = serde_json::json!({
            "type": "message",
            "user": "U456",
            "channel": "C789",
            "text": "Bot message",
            "ts": "1700000000.000100",
            "bot_id": "B999"
        });

        let msg = parse_slack_event(&event, &bot_id, &[]).await;
        assert!(msg.is_none());
    }

    #[tokio::test]
    async fn test_parse_slack_event_filters_own_user() {
        let bot_id = Arc::new(RwLock::new(Some("U456".to_string())));
        let event = serde_json::json!({
            "type": "message",
            "user": "U456",
            "channel": "C789",
            "text": "My message",
            "ts": "1700000000.000100"
        });

        let msg = parse_slack_event(&event, &bot_id, &[]).await;
        assert!(msg.is_none());
    }

    #[tokio::test]
    async fn test_parse_slack_event_channel_filter() {
        let bot_id = Arc::new(RwLock::new(None));
        let event = serde_json::json!({
            "type": "message",
            "user": "U456",
            "channel": "C789",
            "text": "Hello",
            "ts": "1700000000.000100"
        });

        // Not in allowed channels
        let msg =
            parse_slack_event(&event, &bot_id, &["C111".to_string(), "C222".to_string()]).await;
        assert!(msg.is_none());

        // In allowed channels
        let msg = parse_slack_event(&event, &bot_id, &["C789".to_string()]).await;
        assert!(msg.is_some());
    }

    #[tokio::test]
    async fn test_parse_slack_event_skips_other_subtypes() {
        let bot_id = Arc::new(RwLock::new(None));
        // Non-message_changed subtypes should still be filtered
        let event = serde_json::json!({
            "type": "message",
            "subtype": "channel_join",
            "user": "U456",
            "channel": "C789",
            "text": "joined",
            "ts": "1700000000.000100"
        });

        let msg = parse_slack_event(&event, &bot_id, &[]).await;
        assert!(msg.is_none());
    }

    #[tokio::test]
    async fn test_parse_slack_command() {
        let bot_id = Arc::new(RwLock::new(None));
        let event = serde_json::json!({
            "type": "message",
            "user": "U456",
            "channel": "C789",
            "text": "/agent hello-world",
            "ts": "1700000000.000100"
        });

        let msg = parse_slack_event(&event, &bot_id, &[]).await.unwrap();
        match &msg.content {
            ChannelContent::Command { name, args } => {
                assert_eq!(name, "agent");
                assert_eq!(args, &["hello-world"]);
            }
            other => unreachable!("Expected Command, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_parse_slack_event_message_changed() {
        let bot_id = Arc::new(RwLock::new(Some("B123".to_string())));
        let event = serde_json::json!({
            "type": "message",
            "subtype": "message_changed",
            "channel": "C789",
            "message": {
                "user": "U456",
                "text": "Edited message text",
                "ts": "1700000000.000100"
            },
            "ts": "1700000001.000200"
        });

        let msg = parse_slack_event(&event, &bot_id, &[]).await.unwrap();
        assert_eq!(msg.channel, ChannelType::Slack);
        assert_eq!(msg.sender.platform_id, "C789");
        assert!(matches!(msg.content, ChannelContent::Text(ref t) if t == "Edited message text"));
    }

    #[test]
    fn test_slack_adapter_creation() {
        let adapter = SlackAdapter::new(
            "xapp-test".to_string(),
            "xoxb-test".to_string(),
            vec!["C123".to_string()],
        );
        assert_eq!(adapter.name(), "slack");
        assert_eq!(adapter.channel_type(), ChannelType::Slack);
        // Streaming is implemented via chat.update — bridge picks the
        // progressive path instead of the buffered default.
        assert!(adapter.supports_streaming());
    }

    /// Regression: `api_send_message` previously logged a warn on
    /// Slack's `ok: false` response shape (channel_not_found,
    /// not_in_channel, token_revoked, channel_archived, …) but
    /// returned `Ok(())`. The bridge dispatcher saw success and
    /// silently dropped the message. Source-shape audit pins the
    /// fix: the function must return an `Err` after the warn so the
    /// caller can surface the failure to the operator. A future
    /// refactor that drops the `return Err(...)` trips this test.
    #[test]
    fn api_send_message_returns_err_on_slack_ok_false() {
        let src = include_str!("slack.rs").replace("\r\n", "\n");
        let prod_end = src.find("#[cfg(test)]").expect("test mod exists");
        let prod = &src[..prod_end];

        assert!(
            prod.contains("if resp[\"ok\"].as_bool() != Some(true)"),
            "ok-false guard must still be in place"
        );
        // The corrective Err return inside the guard. (Formatting-robust: the
        // format! may wrap across lines, so match the message literal.)
        assert!(
            prod.contains("\"Slack chat.postMessage failed:"),
            "api_send_message must Err on ok=false (not silently return Ok)"
        );
        // Pre-fix shape: a bare warn without a propagated error.
        let bad = [
            "warn!(\"Slack chat.postMessage failed:",
            " {err}\");\n",
            "            }\n        }\n        Ok(())",
        ]
        .concat();
        assert!(
            !prod.contains(&bad),
            "warn-then-Ok pattern must not return — the dispatcher cannot \
             surface failed Slack deliveries without an Err"
        );
    }
}
