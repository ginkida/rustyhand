//! Channel Bridge Layer for the RustyHand Agent OS.
//!
//! Provides messaging integrations that convert platform messages into
//! unified `ChannelMessage` events for the kernel. RustyHand ships
//! adapters for the three platforms whose APIs work without a public
//! webhook URL — Telegram (long-polling), Discord (Gateway WebSocket),
//! and Slack (Socket Mode).

pub mod bridge;
pub mod discord;
pub mod formatter;
pub mod router;
pub mod slack;
pub mod telegram;
pub mod types;
