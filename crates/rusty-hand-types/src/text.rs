//! UTF-8-safe string utilities.
//!
//! Naive byte slicing (`&s[..N]`) panics when N falls inside a multi-byte
//! UTF-8 character. These helpers truncate safely on character boundaries.

/// Truncate a string to at most `max_bytes` bytes, stopping at a character
/// boundary. Returns the original string if it's already short enough.
///
/// Unlike `&s[..max_bytes]`, this never panics on non-ASCII input.
pub fn truncate_bytes(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Find the largest valid char boundary <= max_bytes.
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Snap a byte index down to the nearest UTF-8 char boundary at or below
/// `idx`. Returns `s.len()` if `idx >= s.len()`.
///
/// Use this when you need to slice `s[a..b]` after computing `a`/`b` via
/// arithmetic on positions (e.g. `pos.saturating_sub(60)`, `pos + 60`) —
/// arithmetic positions can land mid-character.
pub fn floor_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut end = idx;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    end
}

/// Redact the Telegram bot token segment from any string (typically a
/// `reqwest::Error::Display` output).
///
/// Telegram's REST API embeds the bot token in the URL path:
/// `https://api.telegram.org/bot{TOKEN}/sendMessage`. reqwest's
/// `Error::Display` includes the request URL on failure, so logging
/// the raw error leaks the bot token to any consumer that scrapes
/// the daemon's stderr (journalctl, syslog, Loki, Vector, etc.).
///
/// This helper finds every `telegram.org/bot…/` segment and replaces
/// the token (between `bot` and the next `/`) with `<redacted>`.
/// Returns the original string unchanged if no token pattern is found.
///
/// Shared with the kernel's cron-channel delivery and the
/// rusty-hand-channels Telegram adapter — both build the same URL
/// shape and both used to leak tokens via tracing::warn on send
/// failure.
pub fn redact_telegram_token(s: &str) -> String {
    // Telegram embeds the token after `/bot` in two URL shapes:
    //   api.telegram.org/bot{TOKEN}/sendMessage      (Bot API)
    //   api.telegram.org/file/bot{TOKEN}/{path}      (file download)
    // Anchor on the host, then redact the segment between the next `/bot` and
    // the following `/` (or end of string). This covers both shapes; a plain
    // `telegram.org/bot` prefix match would miss the `/file/bot` download URL.
    const DOMAIN: &str = "telegram.org";
    const BOT: &str = "/bot";
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(dom_idx) = rest.find(DOMAIN) {
        let after_dom_start = dom_idx + DOMAIN.len();
        let after_dom = &rest[after_dom_start..];
        match after_dom.find(BOT) {
            Some(bot_rel) => {
                // Emit everything up to and including the `/bot` marker.
                let token_start = bot_rel + BOT.len();
                out.push_str(&rest[..after_dom_start + token_start]);
                let after_bot = &after_dom[token_start..];
                match after_bot.find('/') {
                    Some(slash) => {
                        out.push_str("<redacted>");
                        rest = &after_bot[slash..];
                    }
                    None => {
                        out.push_str("<redacted>");
                        rest = "";
                        break;
                    }
                }
            }
            None => {
                // Domain with no `/bot` after it — emit through the domain and
                // continue scanning (advances `rest`, so no infinite loop).
                out.push_str(&rest[..after_dom_start]);
                rest = after_dom;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Truncate a string to at most `max_chars` characters.
/// Unlike byte-based slicing, this always produces valid UTF-8.
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect()
}

/// Truncate with ellipsis suffix if the string exceeds `max_chars`.
pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    let total = s.chars().count();
    if total <= max_chars {
        return s.to_string();
    }
    let truncated: String = s.chars().take(max_chars).collect();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_bytes_ascii() {
        assert_eq!(truncate_bytes("hello world", 5), "hello");
        assert_eq!(truncate_bytes("hi", 100), "hi");
        assert_eq!(truncate_bytes("", 10), "");
    }

    #[test]
    fn truncate_bytes_multibyte_never_panics() {
        // Each emoji is 4 bytes in UTF-8
        let s = "hello 🦀 world 🦀 rust";
        // Slicing mid-emoji would panic with naive `&s[..N]`
        for n in 0..s.len() {
            let out = truncate_bytes(s, n);
            // Must always be valid UTF-8 (no panic, no truncated char)
            assert!(out.is_char_boundary(out.len()));
            assert!(out.len() <= n);
        }
    }

    #[test]
    fn truncate_bytes_russian() {
        // Cyrillic characters are 2 bytes each
        let s = "Привет мир"; // 19 bytes, 10 chars
        assert_eq!(truncate_bytes(s, 100), s);
        // Should back off to previous boundary if mid-char
        let out = truncate_bytes(s, 5);
        assert!(out.len() <= 5);
        assert!(std::str::from_utf8(out.as_bytes()).is_ok());
    }

    #[test]
    fn truncate_chars_basic() {
        assert_eq!(truncate_chars("hello", 3), "hel");
        assert_eq!(truncate_chars("hi", 10), "hi");
        assert_eq!(truncate_chars("🦀🦀🦀", 2), "🦀🦀");
    }

    #[test]
    fn floor_char_boundary_ascii() {
        let s = "hello world";
        for i in 0..=s.len() {
            assert_eq!(floor_char_boundary(s, i), i);
        }
        assert_eq!(floor_char_boundary(s, 999), s.len());
    }

    #[test]
    fn floor_char_boundary_multibyte() {
        // Cyrillic: each char is 2 bytes
        let s = "Привет"; // 12 bytes, 6 chars
        assert_eq!(floor_char_boundary(s, 0), 0);
        assert_eq!(floor_char_boundary(s, 1), 0); // mid-char, snap down
        assert_eq!(floor_char_boundary(s, 2), 2); // boundary
        assert_eq!(floor_char_boundary(s, 3), 2); // mid-char, snap down
        assert_eq!(floor_char_boundary(s, 12), 12);
        assert_eq!(floor_char_boundary(s, 100), s.len());
        // Slicing at the result must never panic
        for i in 0..=20 {
            let _ = &s[..floor_char_boundary(s, i)];
        }
    }

    #[test]
    fn truncate_with_ellipsis_works() {
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hello...");
        assert_eq!(truncate_with_ellipsis("hi", 10), "hi");
        assert_eq!(truncate_with_ellipsis("🦀🦀🦀🦀🦀", 3), "🦀🦀🦀...");
    }

    #[test]
    fn redact_telegram_token_strips_token_from_url() {
        let raw = "error sending request for url (https://api.telegram.org/bot1234567890:ABCDEF_secret_token/sendMessage): connection refused";
        let red = redact_telegram_token(raw);
        assert!(!red.contains("1234567890:ABCDEF_secret_token"));
        assert!(red.contains("api.telegram.org/bot<redacted>/sendMessage"));
        assert!(red.contains("connection refused"));
    }

    #[test]
    fn redact_telegram_token_strips_token_from_file_download_url() {
        // The file-download URL shape is .../file/bot{TOKEN}/{path} — the old
        // `telegram.org/bot` prefix matcher missed it, leaking the token.
        let raw = "Media download failed: error sending request for url (https://api.telegram.org/file/bot1234567890:ABCDEF_secret/photos/file_1.jpg): timed out";
        let red = redact_telegram_token(raw);
        assert!(!red.contains("1234567890:ABCDEF_secret"));
        assert!(red.contains("api.telegram.org/file/bot<redacted>/photos/file_1.jpg"));
        assert!(red.contains("timed out"));
    }

    #[test]
    fn redact_telegram_token_handles_multiple_occurrences() {
        let raw = "redirect from https://api.telegram.org/botABC/sendMessage to https://api.telegram.org/botXYZ/getMe";
        let red = redact_telegram_token(raw);
        assert!(!red.contains("botABC/"));
        assert!(!red.contains("botXYZ/"));
        assert_eq!(red.matches("bot<redacted>").count(), 2);
    }

    #[test]
    fn redact_telegram_token_passthrough_for_unrelated() {
        let raw = "error sending request for url (https://api.openai.com/v1/embeddings): timeout";
        assert_eq!(redact_telegram_token(raw), raw);
    }

    #[test]
    fn redact_telegram_token_handles_trailing_token_with_no_slash() {
        // Edge case: error truncated mid-URL with no trailing `/`. The
        // token must still be redacted (don't leave it dangling).
        let raw = "https://api.telegram.org/bot1234:secret";
        let red = redact_telegram_token(raw);
        assert!(red.ends_with("bot<redacted>"));
        assert!(!red.contains("1234:secret"));
    }
}
