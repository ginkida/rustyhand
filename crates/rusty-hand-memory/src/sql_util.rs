//! Small SQL helper utilities shared across memory stores.

/// Escape a user-supplied query string for safe use in a SQLite `LIKE`
/// pattern.
///
/// SQLite treats `%` (any sequence of chars) and `_` (any single char) as
/// LIKE wildcards. Without escaping, a user searching their session
/// history for `"100%"` would match anything containing `"100"` (the `%`
/// would silently expand to "any chars"). Same for `"node_modules"` —
/// the `_` would match any single char. False positives in search
/// results, not a security issue.
///
/// The returned pattern uses `\` as its ESCAPE character — callers must
/// add `ESCAPE '\'` (or `ESCAPE '\\'` in Rust string form) to the SQL
/// clause. The helper escapes the three relevant chars (`\`, `%`, `_`)
/// by prefixing them with `\`.
pub fn escape_like_pattern(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(c);
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_percent_and_underscore() {
        assert_eq!(escape_like_pattern("100%"), r"100\%");
        assert_eq!(escape_like_pattern("node_modules"), r"node\_modules");
        assert_eq!(escape_like_pattern("a%b_c"), r"a\%b\_c");
    }

    #[test]
    fn escapes_backslash_itself() {
        // Otherwise users with literal backslashes in their query would
        // accidentally escape the NEXT char of the actual content.
        assert_eq!(escape_like_pattern(r"path\to"), r"path\\to");
    }

    #[test]
    fn passes_safe_strings_through() {
        assert_eq!(escape_like_pattern("hello world"), "hello world");
        assert_eq!(escape_like_pattern("привет"), "привет");
        assert_eq!(escape_like_pattern(""), "");
    }
}
