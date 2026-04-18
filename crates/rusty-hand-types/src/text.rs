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
    fn truncate_with_ellipsis_works() {
        assert_eq!(truncate_with_ellipsis("hello world", 5), "hello...");
        assert_eq!(truncate_with_ellipsis("hi", 10), "hi");
        assert_eq!(truncate_with_ellipsis("🦀🦀🦀🦀🦀", 3), "🦀🦀🦀...");
    }
}
