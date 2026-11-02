//! Defensive redaction for anything echoed to stdout/stderr/logs.
//!
//! Belt-and-suspenders alongside the config/secrets discipline: a base58
//! 87/88-char key (ed25519 secret / JSON keypair array), a `private_key=...`
//! assignment, or a 64-byte hex blob is masked before display.

/// Mask a string defensively. Idempotent.
pub fn redact(input: &str) -> String {
    let mut out = input.to_string();
    // Key-value assignments with secret-looking names.
    out = kv_pattern(&out, "private_key");
    out = kv_pattern(&out, "secret");
    out = kv_pattern(&out, "api_key");
    out = kv_pattern(&out, "token");
    // 87/88-char base58 runs (Solana secret keys / keypair arrays).
    out = base58_key_runs(&out);
    // 128-hex-char runs (64-byte secrets in hex).
    out = hex_key_runs(&out);
    out
}

fn kv_pattern(input: &str, key: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let lower = input.to_lowercase();
    let bytes = input.as_bytes();
    let key_lower = key.to_lowercase();
    let mut i = 0;
    while i < bytes.len() {
        if lower[i..].starts_with(&key_lower) {
            let mut j = i + key_lower.len();
            // skip spaces/tabs around the separator: "k=v", "k = v", "k: v"
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j < bytes.len() && (bytes[j] == b'=' || bytes[j] == b':') {
                j += 1; // separator
                while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                let value_start = j;
                let mut value_end = j;
                while value_end < bytes.len()
                    && !bytes[value_end].is_ascii_whitespace()
                    && value_end - value_start < 512
                {
                    value_end += 1;
                }
                if value_end - value_start >= 8 {
                    out.push_str(key);
                    out.push_str("=[REDACTED]");
                    i = value_end;
                    continue;
                }
            }
        }
        let ch = input[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn base58_key_runs(input: &str) -> String {
    const ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if ALPHABET.contains(chars[i]) {
            let mut j = i;
            while j < chars.len() && ALPHABET.contains(chars[j]) {
                j += 1;
            }
            let run_len = j - i;
            // Solana secret keys: 87 or 88 base58 chars; keypair JSON arrays:
            // 64×(1-3 digits)+commas ≈ 175+ — those contain commas so are not
            // a single run; the 87/88 run covers the common leak shape.
            if (87..=88).contains(&run_len) {
                out.push_str("[REDACTED-BASE58]");
            } else {
                for c in &chars[i..j] {
                    out.push(*c);
                }
            }
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn hex_key_runs(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_hexdigit() {
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j - i >= 64 {
                out.push_str("[REDACTED-HEX]");
            } else {
                for c in &chars[i..j] {
                    out.push(*c);
                }
            }
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_private_key_assignments() {
        let s = "config bad: private_key=4Nd1m0nB8cWgX2fE9pQ7vLkJsT3hZaY6uCr5iOwSdP2gVbN1xA8yU6tR4qK9jH7fD"; // safe-secret-scan: fixture
        let r = redact(s);
        assert!(r.contains("private_key=[REDACTED]"), "got: {r}");
        assert!(!r.contains("4Nd1m0n"), "raw key must not survive: {r}");
    }

    #[test]
    fn masks_case_insensitive_and_colon_forms() {
        let r = redact("PRIVATE_KEY: abcdef1234567890abcdef"); // safe-secret-scan: fixture
        assert!(r.contains("[REDACTED]"), "got: {r}");
    }

    #[test]
    fn masks_87_char_base58_runs() {
        let key = "4".repeat(87);
        let r = redact(&format!("leaked: {key} end"));
        assert!(r.contains("[REDACTED-BASE58]"), "got: {r}");
        assert!(!r.contains(&key));
    }

    #[test]
    fn masks_long_hex_runs() {
        let hex = "a".repeat(64);
        let r = redact(&format!("sig-ish: {hex}"));
        assert!(r.contains("[REDACTED-HEX]"), "got: {r}");
    }

    #[test]
    fn normal_pubkeys_and_text_survive() {
        // Pubkeys are 43-44 base58 chars — under the 87 threshold.
        let normal = "mint 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P ok";
        assert_eq!(redact(normal), normal);
        assert_eq!(redact("hello world"), "hello world");
        // Transaction signatures are 88 base58 chars — they WILL be masked.
        // That is acceptable collateral: signatures are not secrets, but
        // masking them costs little and the 88-char shape is indistinguishable
        // from a secret key in free text.
    }

    #[test]
    fn idempotent() {
        let s = "private_key=abcdefghijklmnopqrstuvwxyz12 more"; // safe-secret-scan: fixture
        assert_eq!(redact(&redact(s)), redact(s));
    }
}
