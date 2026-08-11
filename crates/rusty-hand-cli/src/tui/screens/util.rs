//! Small helpers shared by the TUI screens.

/// Truncate `s` to at most `max` terminal columns, appending `…` when cut.
///
/// `max` is a COLUMN budget, not a byte budget: every caller pairs it with a
/// `{:<N}` pad (`truncate(&a.name, 17)` inside `format!(" {:<18}", …)`), and
/// Rust's fill/align counts chars. Measuring in bytes therefore breaks the
/// column arithmetic for any non-ASCII string — a 13-char Cyrillic agent name is
/// 26 bytes, so a byte-based guard cuts it to ~9 chars and the row under-fills
/// its cell by a third. Counting chars keeps the guard and the pad in the same
/// unit.
///
/// (This is deliberately char-based rather than display-width-based: `ratatui`
/// clips overflow at the widget edge, so the failure mode for a CJK string that
/// is 1 char but 2 columns wide is a slightly short row, not a corrupted frame.
/// Pulling in a `unicode-width` dependency for that is not worth it.)
///
/// Never panics: the previous implementation sliced `&s[..max - 1]` directly,
/// which panicked both on `max == 0` (underflow) and whenever the byte cut
/// landed inside a multi-byte char.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let head: String = s.chars().take(max - 1).collect();
    format!("{head}\u{2026}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_short_strings_through_unchanged() {
        assert_eq!(truncate("abc", 10), "abc");
        assert_eq!(truncate("abc", 3), "abc");
        assert_eq!(truncate("", 0), "");
    }

    #[test]
    fn truncates_ascii_to_the_column_budget() {
        // 1 column is spent on the ellipsis.
        assert_eq!(truncate("abcdef", 4), "abc\u{2026}");
        assert_eq!(truncate("abcdef", 4).chars().count(), 4);
    }

    /// Regression: the pre-fix helper did `&s[..max - 1]`, a byte slice that
    /// panics when the cut lands inside a multi-byte char. Agent names, session
    /// ids, audit details and peer node names are all attacker- or user-supplied
    /// and routinely non-ASCII, so this panicked the whole TUI on render.
    #[test]
    fn does_not_panic_on_multibyte_at_the_cut_boundary() {
        for s in ["Привет мир", "日本語テキスト", "🎉🎉🎉🎉🎉", "café-naïve"]
        {
            for max in 0..12 {
                let out = truncate(s, max);
                assert!(out.chars().count() <= max.max(1));
            }
        }
    }

    /// `max == 0` underflowed `max - 1` in the pre-fix version.
    #[test]
    fn zero_budget_yields_empty_string() {
        assert_eq!(truncate("abcdef", 0), "");
    }

    /// The point of counting chars rather than bytes: a Cyrillic string that
    /// FITS the column budget must be returned whole. A byte-based guard
    /// (`s.len() <= max`) would see 26 bytes > 17 and truncate it needlessly.
    #[test]
    fn non_ascii_within_budget_is_not_truncated() {
        let name = "Аналитик данных"; // 15 chars, 29 bytes
        assert_eq!(name.chars().count(), 15);
        assert!(name.len() > 17);
        assert_eq!(truncate(name, 17), name);
    }
}
