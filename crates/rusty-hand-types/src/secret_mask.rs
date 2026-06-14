//! Shared TOML secret masking.
//!
//! A single byte-robust masker used by both the HTTP config export
//! (`GET /api/config/export`) and the MCP `config_get` tool, so the two views
//! stay identical. The masker redacts any `key = "..."` pair whose key matches
//! a secret pattern — at ANY nesting depth, including inside inline tables and
//! arrays-of-tables (e.g. `fallback_models = [{ provider="x", api_key="sk" }]`).
//!
//! Only quoted string values are redacted: numbers, booleans, arrays, and
//! inline-table braces are never secrets in RustyHand's config schema, and
//! redacting them would produce malformed TOML.

/// Whether a TOML key name denotes a secret value. Case-insensitive.
pub fn is_secret_key_name(k: &str) -> bool {
    let k = k.to_lowercase();
    k == "api_key"
        || k == "password"
        || k == "token"
        || k == "secret"
        || k == "bearer_token"
        || k.ends_with("_key")
        || k.ends_with("_token")
        || k.ends_with("_password")
        || k.ends_with("_secret")
}

/// Mask every secret-keyed quoted string in a multi-line TOML document.
/// Newline behaviour: one `\n` per input line (matching both prior callers).
pub fn mask_toml_secrets(toml_text: &str) -> String {
    let mut out = String::with_capacity(toml_text.len());
    for line in toml_text.lines() {
        out.push_str(&mask_secrets_in_line(line));
        out.push('\n');
    }
    out
}

/// Walk a single TOML line and redact any `key = "..."` pair whose key matches
/// a secret pattern. Preserves everything else (whitespace, comments, braces).
pub fn mask_secrets_in_line(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0usize;
    while i < bytes.len() {
        // Find the next `ident = "...something..."` pair.
        let key_start = match find_ident_start(bytes, i) {
            Some(k) => k,
            None => {
                out.push_str(&line[i..]);
                break;
            }
        };
        out.push_str(&line[i..key_start]);
        let key_end = find_ident_end(bytes, key_start);
        let key = &line[key_start..key_end];

        // After ident, skip whitespace and check for `=`.
        let mut j = key_end;
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b'=' {
            // Not a key=value pair — emit the ident and continue.
            out.push_str(key);
            i = key_end;
            continue;
        }
        // Skip `=` and any whitespace.
        let eq = j;
        j += 1;
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }
        // Only redact when the value is a quoted string.
        if j >= bytes.len() || bytes[j] != b'"' {
            out.push_str(&line[key_start..=eq]);
            i = eq + 1;
            continue;
        }
        // Find the matching closing quote, respecting `\"` escapes.
        let value_start = j;
        let mut k = j + 1;
        while k < bytes.len() {
            if bytes[k] == b'\\' && k + 1 < bytes.len() {
                k += 2;
                continue;
            }
            if bytes[k] == b'"' {
                break;
            }
            k += 1;
        }
        let value_end = if k < bytes.len() { k + 1 } else { k };

        if is_secret_key_name(key) {
            out.push_str(&line[key_start..value_start]);
            out.push_str("\"<redacted>\"");
        } else {
            out.push_str(&line[key_start..value_end]);
        }
        i = value_end;
    }
    out
}

/// Find the start of the next bare TOML identifier (letters/digits/`_`/`-`).
/// Identifiers only start where the preceding byte is whitespace, `{`, `,`, or
/// start-of-line — otherwise we're mid-value (e.g. inside a string).
fn find_ident_start(bytes: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i < bytes.len() {
        let c = bytes[i];
        // An identifier only starts where the preceding byte is whitespace,
        // `{`, `,`, or start-of-line — otherwise we're mid-value (in a string).
        if (c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            && (i == from || matches!(bytes[i - 1], b' ' | b'\t' | b'{' | b',' | b'\n'))
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_ident_end(bytes: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' {
            i += 1;
        } else {
            break;
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_top_level_secret() {
        let out = mask_toml_secrets("api_key = \"sk-secret\"\nname = \"bob\"\n");
        assert!(out.contains("api_key = \"<redacted>\""));
        assert!(out.contains("name = \"bob\""));
    }

    #[test]
    fn masks_inline_table_and_array_secrets() {
        // The leading key (fallback_models) is not a secret, but the nested
        // api_key inside the inline table must still be redacted.
        let input = "fallback_models = [{ provider = \"kimi\", api_key = \"sk-A\" }]\n";
        let out = mask_toml_secrets(input);
        assert!(out.contains("provider = \"kimi\""), "got: {out}");
        assert!(out.contains("api_key = \"<redacted>\""), "got: {out}");
        assert!(!out.contains("sk-A"), "secret leaked: {out}");
    }

    #[test]
    fn preserves_non_secret_and_non_string_values() {
        let input = "port = 4200\nenabled = true\nname = \"x\"\n";
        let out = mask_toml_secrets(input);
        assert!(out.contains("port = 4200"));
        assert!(out.contains("enabled = true"));
        assert!(out.contains("name = \"x\""));
    }

    #[test]
    fn secret_suffix_keys() {
        assert!(is_secret_key_name("api_key"));
        assert!(is_secret_key_name("bot_token"));
        assert!(is_secret_key_name("Shared_Secret"));
        assert!(!is_secret_key_name("api_key_hash"));
        assert!(!is_secret_key_name("api_key_env"));
        assert!(!is_secret_key_name("name"));
    }
}
