//! Discord Gateway adapter for the RustyHand channel bridge.
//!
//! Uses Discord Gateway WebSocket (v10) for receiving messages and the REST API
//! for sending responses. No external Discord crate — just `tokio-tungstenite` + `reqwest`.

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

const DISCORD_API_BASE: &str = "https://discord.com/api/v10";
const MAX_BACKOFF: Duration = Duration::from_secs(60);
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const DISCORD_MSG_LIMIT: usize = 2000;

/// Discord Gateway opcodes.
mod opcode {
    pub const DISPATCH: u64 = 0;
    pub const HEARTBEAT: u64 = 1;
    pub const IDENTIFY: u64 = 2;
    pub const RESUME: u64 = 6;
    pub const RECONNECT: u64 = 7;
    pub const INVALID_SESSION: u64 = 9;
    pub const HELLO: u64 = 10;
    pub const HEARTBEAT_ACK: u64 = 11;
}

/// Discord Gateway adapter using WebSocket.
pub struct DiscordAdapter {
    /// SECURITY: Bot token is zeroized on drop to prevent memory disclosure.
    token: Zeroizing<String>,
    client: reqwest::Client,
    allowed_guilds: Vec<u64>,
    intents: u64,
    shutdown_tx: Arc<watch::Sender<bool>>,
    shutdown_rx: watch::Receiver<bool>,
    /// Bot's own user ID (populated after READY event).
    bot_user_id: Arc<RwLock<Option<String>>>,
    /// Session ID for resume (populated after READY event).
    session_id: Arc<RwLock<Option<String>>>,
    /// Resume gateway URL.
    resume_gateway_url: Arc<RwLock<Option<String>>>,
}

impl DiscordAdapter {
    pub fn new(token: String, allowed_guilds: Vec<u64>, intents: u64) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            token: Zeroizing::new(token),
            client: reqwest::Client::new(),
            allowed_guilds,
            intents,
            shutdown_tx: Arc::new(shutdown_tx),
            shutdown_rx,
            bot_user_id: Arc::new(RwLock::new(None)),
            session_id: Arc::new(RwLock::new(None)),
            resume_gateway_url: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the WebSocket gateway URL from the Discord API.
    async fn get_gateway_url(&self) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{DISCORD_API_BASE}/gateway/bot");
        let resp: serde_json::Value = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.token.as_str()))
            .send()
            .await?
            .json()
            .await?;

        let ws_url = resp["url"]
            .as_str()
            .ok_or("Missing 'url' in gateway response")?;

        Ok(format!("{ws_url}/?v=10&encoding=json"))
    }

    /// Send a message to a Discord channel via REST API.
    async fn api_send_message(
        &self,
        channel_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{DISCORD_API_BASE}/channels/{channel_id}/messages");
        let chunks = split_message(text, DISCORD_MSG_LIMIT);

        for chunk in chunks {
            let body = serde_json::json!({ "content": chunk });
            let resp = self
                .client
                .post(&url)
                .header("Authorization", format!("Bot {}", self.token.as_str()))
                .json(&body)
                .send()
                .await?;

            // Discord returns 4xx for missing-access (403), unknown-channel
            // (404), rate-limit (429), bot-was-kicked (403), missing-
            // permissions (403). Pre-fix this warn-logged the body and
            // silently returned Ok(()) — the bridge dispatcher believed
            // the message was delivered. Same silent-failure class as
            // Slack and the Telegram fallback paths. Return an Err so
            // the caller can surface the failure to the operator.
            if !resp.status().is_success() {
                let status = resp.status();
                let body_text = resp
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("<failed to read body: {e}>"));
                warn!(
                    channel_id = %channel_id,
                    status = %status,
                    body = %body_text,
                    "Discord sendMessage failed"
                );
                return Err(format!(
                    "Discord sendMessage failed: {status} — {body_text}{}",
                    crate::http_status_hint(status.as_u16())
                )
                .into());
            }
        }
        Ok(())
    }

    /// Send a single message and return its Discord message ID, for streaming
    /// edits. Truncates to the 2000-char limit (overflow is handled by the
    /// streaming caller's final flush).
    async fn api_send_message_returning_id(
        &self,
        channel_id: &str,
        text: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let url = format!("{DISCORD_API_BASE}/channels/{channel_id}/messages");
        let display: String = text.chars().take(DISCORD_MSG_LIMIT).collect();
        let body = serde_json::json!({ "content": display });
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token.as_str()))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {e}>"));
            return Err(format!(
                "Discord sendMessage failed: {status} — {body_text}{}",
                crate::http_status_hint(status.as_u16())
            )
            .into());
        }
        let json: serde_json::Value = resp.json().await?;
        match json["id"].as_str() {
            Some(id) => Ok(id.to_string()),
            None => Err("Discord sendMessage: no message id in response".into()),
        }
    }

    /// Edit an existing message's content (for streaming). Truncates to limit.
    async fn api_edit_message(
        &self,
        channel_id: &str,
        message_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{DISCORD_API_BASE}/channels/{channel_id}/messages/{message_id}");
        let display: String = text.chars().take(DISCORD_MSG_LIMIT).collect();
        let body = serde_json::json!({ "content": display });
        let resp = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bot {}", self.token.as_str()))
            .json(&body)
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {e}>"));
            return Err(format!(
                "Discord editMessage failed: {status} — {body_text}{}",
                crate::http_status_hint(status.as_u16())
            )
            .into());
        }
        Ok(())
    }

    /// Send typing indicator to a Discord channel.
    async fn api_send_typing(&self, channel_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{DISCORD_API_BASE}/channels/{channel_id}/typing");
        let _ = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token.as_str()))
            .send()
            .await?;
        Ok(())
    }
}

#[async_trait]
impl ChannelAdapter for DiscordAdapter {
    fn name(&self) -> &str {
        "discord"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Discord
    }

    async fn start(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>, Box<dyn std::error::Error>>
    {
        let gateway_url = self.get_gateway_url().await?;
        info!("Discord gateway URL obtained");

        // SECURITY: warn if allowed_guilds is empty (bot accepts messages from any server)
        if self.allowed_guilds.is_empty() {
            warn!(
                "Discord bot has no allowed_guilds configured — \
                 messages from any server will be accepted. \
                 Set [channels.discord].allowed_guilds in config to restrict access."
            );
        }

        let (tx, rx) = mpsc::channel::<ChannelMessage>(256);

        let token = self.token.clone();
        let intents = self.intents;
        let allowed_guilds = self.allowed_guilds.clone();
        let bot_user_id = self.bot_user_id.clone();
        let session_id_store = self.session_id.clone();
        let resume_url_store = self.resume_gateway_url.clone();
        let mut shutdown = self.shutdown_rx.clone();

        tokio::spawn(async move {
            let mut backoff = INITIAL_BACKOFF;
            let mut connect_url = gateway_url;
            // Sequence persists across reconnections for RESUME
            let sequence: Arc<RwLock<Option<u64>>> = Arc::new(RwLock::new(None));

            loop {
                if *shutdown.borrow() {
                    break;
                }

                info!("Connecting to Discord gateway...");

                let ws_result = tokio_tungstenite::connect_async(&connect_url).await;
                let ws_stream = match ws_result {
                    Ok((stream, _)) => stream,
                    Err(e) => {
                        warn!("Discord gateway connection failed: {e}, retrying in {backoff:?}");
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(MAX_BACKOFF);
                        continue;
                    }
                };

                backoff = INITIAL_BACKOFF;
                info!("Discord gateway connected");

                let (mut ws_tx, mut ws_rx) = ws_stream.split();
                let mut heartbeat_interval: Option<u64> = None;
                // Periodic heartbeat ticker. Discord requires an op-1 heartbeat
                // every heartbeat_interval ms (delivered in HELLO); without it
                // the gateway treats the connection as a zombie and closes it
                // (~45s). Placeholder period until HELLO arrives; the tick arm
                // is a no-op until heartbeat_interval is set.
                let mut heartbeat =
                    tokio::time::interval(std::time::Duration::from_millis(45_000));
                heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                heartbeat.reset(); // don't fire immediately

                // Inner message loop — returns true if we should reconnect
                let should_reconnect = 'inner: loop {
                    let msg = tokio::select! {
                        msg = ws_rx.next() => msg,
                        _ = heartbeat.tick() => {
                            if heartbeat_interval.is_none() {
                                continue; // HELLO not yet received
                            }
                            let seq = *sequence.read().await;
                            let hb = serde_json::json!({ "op": opcode::HEARTBEAT, "d": seq });
                            if let Err(e) = ws_tx
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    serde_json::to_string(&hb).unwrap(),
                                ))
                                .await
                            {
                                warn!("Discord: failed to send heartbeat: {e}");
                                break 'inner true;
                            }
                            debug!("Discord: sent heartbeat (seq={seq:?})");
                            continue;
                        }
                        _ = shutdown.changed() => {
                            if *shutdown.borrow() {
                                info!("Discord shutdown requested");
                                let _ = ws_tx.close().await;
                                return;
                            }
                            continue;
                        }
                    };

                    let msg = match msg {
                        Some(Ok(m)) => m,
                        Some(Err(e)) => {
                            warn!("Discord WebSocket error: {e}");
                            break 'inner true;
                        }
                        None => {
                            info!("Discord WebSocket closed");
                            break 'inner true;
                        }
                    };

                    let text = match msg {
                        tokio_tungstenite::tungstenite::Message::Text(t) => t,
                        tokio_tungstenite::tungstenite::Message::Close(_) => {
                            info!("Discord gateway closed by server");
                            break 'inner true;
                        }
                        _ => continue,
                    };

                    let payload: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(v) => v,
                        Err(e) => {
                            warn!("Discord: failed to parse gateway message: {e}");
                            continue;
                        }
                    };

                    let op = payload["op"].as_u64().unwrap_or(999);

                    // Update sequence number
                    if let Some(s) = payload["s"].as_u64() {
                        *sequence.write().await = Some(s);
                    }

                    match op {
                        opcode::HELLO => {
                            let interval =
                                payload["d"]["heartbeat_interval"].as_u64().unwrap_or(45000);
                            heartbeat_interval = Some(interval);
                            // Start ticking at the server-provided interval; the
                            // first heartbeat goes out one interval after HELLO.
                            heartbeat = tokio::time::interval(
                                std::time::Duration::from_millis(interval.max(1)),
                            );
                            heartbeat.set_missed_tick_behavior(
                                tokio::time::MissedTickBehavior::Delay,
                            );
                            heartbeat.reset();
                            debug!("Discord HELLO: heartbeat_interval={interval}ms");

                            // Try RESUME if we have a session, otherwise IDENTIFY
                            let has_session = session_id_store.read().await.is_some();
                            let has_seq = sequence.read().await.is_some();

                            let gateway_msg = if has_session && has_seq {
                                let sid = session_id_store.read().await.clone().unwrap();
                                let seq = *sequence.read().await;
                                info!("Discord: sending RESUME (session={sid})");
                                serde_json::json!({
                                    "op": opcode::RESUME,
                                    "d": {
                                        "token": token.as_str(),
                                        "session_id": sid,
                                        "seq": seq
                                    }
                                })
                            } else {
                                info!("Discord: sending IDENTIFY");
                                serde_json::json!({
                                    "op": opcode::IDENTIFY,
                                    "d": {
                                        "token": token.as_str(),
                                        "intents": intents,
                                        "properties": {
                                            "os": "linux",
                                            "browser": "rustyhand",
                                            "device": "rustyhand"
                                        }
                                    }
                                })
                            };

                            if let Err(e) = ws_tx
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    serde_json::to_string(&gateway_msg).unwrap(),
                                ))
                                .await
                            {
                                error!("Discord: failed to send IDENTIFY/RESUME: {e}");
                                break 'inner true;
                            }
                        }

                        opcode::DISPATCH => {
                            let event_name = payload["t"].as_str().unwrap_or("");
                            let d = &payload["d"];

                            match event_name {
                                "READY" => {
                                    let user_id =
                                        d["user"]["id"].as_str().unwrap_or("").to_string();
                                    let username =
                                        d["user"]["username"].as_str().unwrap_or("unknown");
                                    let sid = d["session_id"].as_str().unwrap_or("").to_string();
                                    let resume_url =
                                        d["resume_gateway_url"].as_str().unwrap_or("").to_string();

                                    *bot_user_id.write().await = Some(user_id.clone());
                                    *session_id_store.write().await = Some(sid);
                                    if !resume_url.is_empty() {
                                        *resume_url_store.write().await = Some(resume_url);
                                    }

                                    info!("Discord bot ready: {username} ({user_id})");
                                }

                                "MESSAGE_CREATE" | "MESSAGE_UPDATE" => {
                                    if let Some(msg) =
                                        parse_discord_message(d, &bot_user_id, &allowed_guilds)
                                            .await
                                    {
                                        debug!(
                                            "Discord {event_name} from channel={}: {:?}",
                                            msg.sender.platform_id, msg.content
                                        );
                                        if tx.send(msg).await.is_err() {
                                            return;
                                        }
                                    }
                                }

                                "RESUMED" => {
                                    info!("Discord session resumed successfully");
                                }

                                _ => {
                                    debug!("Discord event: {event_name}");
                                }
                            }
                        }

                        opcode::HEARTBEAT => {
                            // Server requests immediate heartbeat
                            let seq = *sequence.read().await;
                            let hb = serde_json::json!({ "op": opcode::HEARTBEAT, "d": seq });
                            let _ = ws_tx
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    serde_json::to_string(&hb).unwrap(),
                                ))
                                .await;
                        }

                        opcode::HEARTBEAT_ACK => {
                            debug!("Discord heartbeat ACK received");
                        }

                        opcode::RECONNECT => {
                            info!("Discord: server requested reconnect");
                            break 'inner true;
                        }

                        opcode::INVALID_SESSION => {
                            let resumable = payload["d"].as_bool().unwrap_or(false);
                            if resumable {
                                info!("Discord: invalid session (resumable)");
                            } else {
                                info!("Discord: invalid session (not resumable), clearing session");
                                *session_id_store.write().await = None;
                                *sequence.write().await = None;
                            }
                            break 'inner true;
                        }

                        _ => {
                            debug!("Discord: unknown opcode {op}");
                        }
                    }
                };

                if !should_reconnect || *shutdown.borrow() {
                    break;
                }

                // Try resume URL if available
                if let Some(ref url) = *resume_url_store.read().await {
                    connect_url = format!("{url}/?v=10&encoding=json");
                }

                warn!("Discord: reconnecting in {backoff:?}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }

            info!("Discord gateway loop stopped");
        });

        let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }

    async fn send(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // platform_id is the channel_id for Discord
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

    async fn send_typing(&self, user: &ChannelUser) -> Result<(), Box<dyn std::error::Error>> {
        self.api_send_typing(&user.platform_id).await
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn send_streaming(
        &self,
        user: &ChannelUser,
        mut rx: tokio::sync::mpsc::Receiver<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // platform_id is the channel_id for Discord
        let channel_id = &user.platform_id;
        let mut full_text = String::new();
        let mut message_id: Option<String> = None;
        let mut last_edit = std::time::Instant::now();

        // Discord's per-route edit rate limits are stricter than Telegram's;
        // throttle to ~1 edit/sec to avoid 429s.
        const EDIT_INTERVAL: Duration = Duration::from_millis(1000);

        while let Some(chunk) = rx.recv().await {
            full_text.push_str(&chunk);

            if message_id.is_none() {
                // Send the first chunk as a new message and capture its ID.
                if !full_text.is_empty() {
                    match self
                        .api_send_message_returning_id(channel_id, &full_text)
                        .await
                    {
                        Ok(mid) => message_id = Some(mid),
                        Err(e) => {
                            warn!("Discord streaming: failed to send initial message: {e}");
                            break;
                        }
                    }
                    last_edit = std::time::Instant::now();
                }
            } else if last_edit.elapsed() >= EDIT_INTERVAL {
                // Throttled mid-stream edit. Failures are recoverable (next
                // tick retries with newer text) — log at debug, don't break.
                if let Some(ref mid) = message_id {
                    if let Err(e) = self.api_edit_message(channel_id, mid, &full_text).await {
                        tracing::debug!(
                            channel_id = %channel_id, error = %e,
                            "Discord streaming: mid-stream edit failed (will retry next tick)"
                        );
                    }
                    last_edit = std::time::Instant::now();
                }
            }
        }

        // Final flush: edit the first message with the complete text, sending
        // any overflow beyond the 2000-char limit as new messages.
        if let Some(ref mid) = message_id {
            if !full_text.is_empty() {
                let chunks = split_message(&full_text, DISCORD_MSG_LIMIT);
                if let Err(e) = self.api_edit_message(channel_id, mid, chunks[0]).await {
                    warn!(
                        channel_id = %channel_id, error = %e,
                        "Discord streaming: final edit failed — user view stale"
                    );
                }
                for extra in &chunks[1..] {
                    if let Err(e) = self.api_send_message(channel_id, extra).await {
                        warn!(
                            channel_id = %channel_id, error = %e,
                            "Discord streaming: overflow chunk send failed — partial response lost"
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

/// Parse a Discord MESSAGE_CREATE or MESSAGE_UPDATE payload into a `ChannelMessage`.
async fn parse_discord_message(
    d: &serde_json::Value,
    bot_user_id: &Arc<RwLock<Option<String>>>,
    allowed_guilds: &[u64],
) -> Option<ChannelMessage> {
    let author = d.get("author")?;
    let author_id = author["id"].as_str()?;

    // Filter out bot's own messages
    if let Some(ref bid) = *bot_user_id.read().await {
        if author_id == bid {
            return None;
        }
    }

    // Filter out other bots
    if author["bot"].as_bool() == Some(true) {
        return None;
    }

    // Discord DMs carry no guild_id. Track this so (a) the bridge applies
    // dm_policy rather than group_policy and (b) a configured guild allowlist
    // also gates DMs instead of letting them slip through unfiltered.
    let in_guild = d["guild_id"].as_str().is_some();

    // Filter by allowed guilds. A non-empty allowlist locks the bot to those
    // guilds; messages from other guilds — and DMs, which belong to no guild —
    // are rejected.
    if !allowed_guilds.is_empty() {
        match d["guild_id"].as_str() {
            Some(guild_id) => {
                let gid: u64 = guild_id.parse().unwrap_or(0);
                if !allowed_guilds.contains(&gid) {
                    return None;
                }
            }
            None => return None, // DM — not part of any allowed guild
        }
    }

    let content_text = d["content"].as_str().unwrap_or("");
    if content_text.is_empty() {
        return None;
    }

    let channel_id = d["channel_id"].as_str()?;
    let message_id = d["id"].as_str().unwrap_or("0");
    let username = author["username"].as_str().unwrap_or("Unknown");
    let discriminator = author["discriminator"].as_str().unwrap_or("0000");
    let display_name = if discriminator == "0" {
        username.to_string()
    } else {
        format!("{username}#{discriminator}")
    };

    let timestamp = d["timestamp"]
        .as_str()
        .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);

    // Parse commands (messages starting with /)
    let content = if content_text.starts_with('/') {
        let parts: Vec<&str> = content_text.splitn(2, ' ').collect();
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
        ChannelContent::Text(content_text.to_string())
    };

    Some(ChannelMessage {
        channel: ChannelType::Discord,
        platform_message_id: message_id.to_string(),
        sender: ChannelUser {
            platform_id: channel_id.to_string(),
            display_name,
            rusty_hand_user: None,
        },
        content,
        target_agent: None,
        timestamp,
        is_group: in_guild,
        thread_id: None,
        metadata: HashMap::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_discord_message_basic() {
        let bot_id = Arc::new(RwLock::new(Some("bot123".to_string())));
        let d = serde_json::json!({
            "id": "msg1",
            "channel_id": "ch1",
            "content": "Hello agent!",
            "author": {
                "id": "user456",
                "username": "alice",
                "discriminator": "0",
                "bot": false
            },
            "timestamp": "2024-01-01T00:00:00+00:00"
        });

        let msg = parse_discord_message(&d, &bot_id, &[]).await.unwrap();
        assert_eq!(msg.channel, ChannelType::Discord);
        assert_eq!(msg.sender.display_name, "alice");
        assert_eq!(msg.sender.platform_id, "ch1");
        assert!(matches!(msg.content, ChannelContent::Text(ref t) if t == "Hello agent!"));
    }

    #[tokio::test]
    async fn test_parse_discord_message_filters_bot() {
        let bot_id = Arc::new(RwLock::new(Some("bot123".to_string())));
        let d = serde_json::json!({
            "id": "msg1",
            "channel_id": "ch1",
            "content": "My own message",
            "author": {
                "id": "bot123",
                "username": "rustyhand",
                "discriminator": "0"
            },
            "timestamp": "2024-01-01T00:00:00+00:00"
        });

        let msg = parse_discord_message(&d, &bot_id, &[]).await;
        assert!(msg.is_none());
    }

    #[tokio::test]
    async fn test_parse_discord_message_filters_other_bots() {
        let bot_id = Arc::new(RwLock::new(Some("bot123".to_string())));
        let d = serde_json::json!({
            "id": "msg1",
            "channel_id": "ch1",
            "content": "Bot message",
            "author": {
                "id": "other_bot",
                "username": "somebot",
                "discriminator": "0",
                "bot": true
            },
            "timestamp": "2024-01-01T00:00:00+00:00"
        });

        let msg = parse_discord_message(&d, &bot_id, &[]).await;
        assert!(msg.is_none());
    }

    #[tokio::test]
    async fn test_parse_discord_message_guild_filter() {
        let bot_id = Arc::new(RwLock::new(Some("bot123".to_string())));
        let d = serde_json::json!({
            "id": "msg1",
            "channel_id": "ch1",
            "guild_id": "999",
            "content": "Hello",
            "author": {
                "id": "user1",
                "username": "bob",
                "discriminator": "0"
            },
            "timestamp": "2024-01-01T00:00:00+00:00"
        });

        // Not in allowed guilds
        let msg = parse_discord_message(&d, &bot_id, &[111, 222]).await;
        assert!(msg.is_none());

        // In allowed guilds
        let msg = parse_discord_message(&d, &bot_id, &[999]).await;
        assert!(msg.is_some());
    }

    #[tokio::test]
    async fn test_parse_discord_dm_classification_and_allowlist() {
        let bot_id = Arc::new(RwLock::new(Some("bot123".to_string())));
        // A DM has no guild_id.
        let dm = serde_json::json!({
            "id": "msg1",
            "channel_id": "dmchan",
            "content": "hi",
            "author": { "id": "user1", "username": "bob", "discriminator": "0" },
            "timestamp": "2024-01-01T00:00:00+00:00"
        });

        // No allowlist: DM is accepted and classified as NOT a group (so the
        // bridge applies dm_policy, not group_policy).
        let msg = parse_discord_message(&dm, &bot_id, &[]).await.unwrap();
        assert!(!msg.is_group, "a DM must not be is_group");

        // With a guild allowlist, DMs (no guild) are rejected — they belong to
        // no allowed guild and must not bypass the allowlist.
        assert!(parse_discord_message(&dm, &bot_id, &[111]).await.is_none());
    }

    #[tokio::test]
    async fn test_parse_discord_guild_message_is_group() {
        let bot_id = Arc::new(RwLock::new(None));
        let d = serde_json::json!({
            "id": "msg1",
            "channel_id": "ch1",
            "guild_id": "999",
            "content": "hi",
            "author": { "id": "user1", "username": "bob", "discriminator": "0" },
            "timestamp": "2024-01-01T00:00:00+00:00"
        });
        let msg = parse_discord_message(&d, &bot_id, &[]).await.unwrap();
        assert!(msg.is_group, "a guild message must be is_group");
    }

    #[tokio::test]
    async fn test_parse_discord_command() {
        let bot_id = Arc::new(RwLock::new(None));
        let d = serde_json::json!({
            "id": "msg1",
            "channel_id": "ch1",
            "content": "/agent hello-world",
            "author": {
                "id": "user1",
                "username": "alice",
                "discriminator": "0"
            },
            "timestamp": "2024-01-01T00:00:00+00:00"
        });

        let msg = parse_discord_message(&d, &bot_id, &[]).await.unwrap();
        match &msg.content {
            ChannelContent::Command { name, args } => {
                assert_eq!(name, "agent");
                assert_eq!(args, &["hello-world"]);
            }
            other => unreachable!("Expected Command, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_parse_discord_empty_content() {
        let bot_id = Arc::new(RwLock::new(None));
        let d = serde_json::json!({
            "id": "msg1",
            "channel_id": "ch1",
            "content": "",
            "author": {
                "id": "user1",
                "username": "alice",
                "discriminator": "0"
            },
            "timestamp": "2024-01-01T00:00:00+00:00"
        });

        let msg = parse_discord_message(&d, &bot_id, &[]).await;
        assert!(msg.is_none());
    }

    #[tokio::test]
    async fn test_parse_discord_discriminator() {
        let bot_id = Arc::new(RwLock::new(None));
        let d = serde_json::json!({
            "id": "msg1",
            "channel_id": "ch1",
            "content": "Hi",
            "author": {
                "id": "user1",
                "username": "alice",
                "discriminator": "1234"
            },
            "timestamp": "2024-01-01T00:00:00+00:00"
        });

        let msg = parse_discord_message(&d, &bot_id, &[]).await.unwrap();
        assert_eq!(msg.sender.display_name, "alice#1234");
    }

    #[tokio::test]
    async fn test_parse_discord_message_update() {
        let bot_id = Arc::new(RwLock::new(Some("bot123".to_string())));
        let d = serde_json::json!({
            "id": "msg1",
            "channel_id": "ch1",
            "content": "Edited message content",
            "author": {
                "id": "user456",
                "username": "alice",
                "discriminator": "0",
                "bot": false
            },
            "timestamp": "2024-01-01T00:00:00+00:00",
            "edited_timestamp": "2024-01-01T00:01:00+00:00"
        });

        // MESSAGE_UPDATE uses the same parse function as MESSAGE_CREATE
        let msg = parse_discord_message(&d, &bot_id, &[]).await.unwrap();
        assert_eq!(msg.channel, ChannelType::Discord);
        assert!(
            matches!(msg.content, ChannelContent::Text(ref t) if t == "Edited message content")
        );
    }

    #[test]
    fn test_discord_adapter_creation() {
        let adapter = DiscordAdapter::new("test-token".to_string(), vec![123, 456], 33280);
        assert_eq!(adapter.name(), "discord");
        assert_eq!(adapter.channel_type(), ChannelType::Discord);
        // Streaming is implemented via PATCH message edits — bridge picks the
        // progressive path instead of the buffered default.
        assert!(adapter.supports_streaming());
    }

    /// Regression: `api_send_message` previously warn-logged non-2xx
    /// Discord responses (missing_access 403, unknown_channel 404,
    /// rate_limit 429, bot-kicked 403) and then silently returned
    /// `Ok(())`. The bridge dispatcher believed every send succeeded.
    /// Same silent-failure class as Slack (iter 56). Source-shape
    /// audit pins the fix: the function must return an `Err` after
    /// the warn so callers can surface failures.
    #[test]
    fn api_send_message_returns_err_on_discord_non_success() {
        let src = include_str!("discord.rs").replace("\r\n", "\n");
        let prod_end = src.find("#[cfg(test)]").expect("test mod exists");
        let prod = &src[..prod_end];

        assert!(
            prod.contains("if !resp.status().is_success()"),
            "non-success guard must still be in place"
        );
        // The corrective Err return inside the guard. (Formatting-robust: the
        // format! may wrap across lines, so match the message literal — which
        // only appears in the Err path; the warn! uses it as a tracing field
        // without the trailing colon.)
        assert!(
            prod.contains("\"Discord sendMessage failed:"),
            "api_send_message must Err on Discord 4xx/5xx (not silently return Ok)"
        );

        // Pre-fix shape: warn-then-Ok without propagating.
        let bad = [
            "warn!(\"Discord sendMessage failed: {body_text}\");\n",
            "            }\n        }\n        Ok(())",
        ]
        .concat();
        assert!(
            !prod.contains(&bad),
            "warn-then-Ok pattern must not return — dispatcher cannot \
             surface failed Discord deliveries without an Err"
        );
    }
}
