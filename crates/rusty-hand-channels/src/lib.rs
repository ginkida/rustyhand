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

/// Human-readable cause for a common channel HTTP failure status (Telegram,
/// Discord), so an operator sees *why* a send failed instead of a bare code.
/// Returns "" for statuses with no specific hint (caller shows raw status+body).
pub(crate) fn http_status_hint(status: u16) -> &'static str {
    match status {
        401 => " — unauthorized: check the bot token",
        403 => " — forbidden: the bot lacks permission in this chat (grant it post/admin rights, or it was removed)",
        404 => " — not found: the chat/channel doesn't exist or the bot was removed",
        429 => " — rate limited: sending too fast, back off and retry",
        500..=599 => " — provider server-side error; retry shortly",
        _ => "",
    }
}

/// Human-readable cause for a common Slack Web API error code (Slack returns
/// HTTP 200 with `{ok:false, error}` rather than an HTTP status). Returns "".
pub(crate) fn slack_error_hint(err: &str) -> &'static str {
    match err {
        "channel_not_found" => " — channel not found (invite the bot or check the ID)",
        "not_in_channel" => " — the bot isn't in this channel; /invite it first",
        "is_archived" => " — the channel is archived",
        "rate_limited" | "ratelimited" => " — rate limited; back off and retry",
        "invalid_auth" | "not_authed" | "token_revoked" | "account_inactive" => {
            " — invalid or expired bot token"
        }
        "msg_too_long" => " — message exceeds Slack's length limit",
        "no_permission" | "restricted_action" => " — the bot lacks permission for this action",
        _ => "",
    }
}

#[cfg(test)]
mod hint_tests {
    use super::*;

    #[test]
    fn http_status_hints_cover_common_failures() {
        assert!(http_status_hint(429).contains("rate limited"));
        assert!(http_status_hint(403).contains("permission"));
        assert!(http_status_hint(404).contains("not found"));
        assert!(http_status_hint(401).contains("token"));
        assert!(http_status_hint(503).contains("server-side"));
        assert_eq!(http_status_hint(200), "");
    }

    #[test]
    fn slack_error_hints_cover_common_codes() {
        assert!(slack_error_hint("channel_not_found").contains("channel"));
        assert!(slack_error_hint("not_in_channel").contains("invite"));
        assert!(slack_error_hint("rate_limited").contains("rate limited"));
        assert!(slack_error_hint("invalid_auth").contains("token"));
        assert_eq!(slack_error_hint("some_unknown_code"), "");
    }
}
