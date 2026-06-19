//! CCR marker generation — 1:1 port of plugins/aphrodite/_marker/marker.py
//!
//! Generates <<<CCR:hash|type|size>>> markers with TOML-driven templates.

use std::collections::HashMap;

/// Check if a string is a valid CCR hash (>=24 hex chars, or i: prefix with >=6 hex).
pub fn is_valid_ccr_hash(h: &str) -> bool {
    if h.len() < 8 {
        return false;
    }
    let h = h.to_lowercase();
    if let Some(stripped) = h.strip_prefix("i:") {
        stripped.len() >= 6 && stripped.chars().all(|c| c.is_ascii_hexdigit())
    } else {
        h.len() >= 24 && h.chars().all(|c| c.is_ascii_hexdigit())
    }
}

/// Build a CCR output block.
/// hash_val: the content hash
/// ccr_type: content type string (e.g. "code_rust", "build")
/// size: original content size in bytes
/// preview: the formatted preview string
/// headroom_budget: optional token budget for truncation
/// meta: optional metadata key-value pairs
/// center: optional center annotation
pub fn ccr_marker(
    hash_val: &str,
    ccr_type: &str,
    size: usize,
    preview: &str,
    headroom_budget: Option<u32>,
    meta: Option<&HashMap<String, String>>,
    center: Option<&str>,
) -> String {
    // Sanitize preview: replace | and newlines
    let mut safe = preview
        .replace('|', "-")
        .replace('\n', " ")
        .replace('\r', " ")
        .trim()
        .to_string();
    // Strip control chars
    safe = safe.chars().filter(|c| *c >= ' ').collect();

    // Headroom budget truncation
    if let Some(budget) = headroom_budget {
        safe = if budget < 25 {
            safe.chars().take(30).collect()
        } else if budget < 50 {
            safe.chars().take(60).collect()
        } else if budget < 75 {
            safe.chars().take(100).collect()
        } else {
            safe
        };
    }

    // Metadata string
    let meta_str = if let Some(m) = meta {
        let parts: Vec<String> = m
            .iter()
            .filter_map(|(k, v)| {
                let sv = v.replace('|', "/").replace('\n', " ").trim().to_string();
                if sv.is_empty() { None } else { Some(format!("{}={}", k, sv)) }
            })
            .collect();
        let mut s = parts.join(";");
        if s.len() > 300 {
            s = format!("{}...", &s[..297]);
        }
        s
    } else {
        String::new()
    };

    // Build marker using the standard template
    render_marker(&safe, ccr_type, &meta_str, center, hash_val, size)
}

/// Render the marker using the canonical three-line format.
fn render_marker(
    preview: &str,
    ccr_type: &str,
    meta: &str,
    center: Option<&str>,
    hash: &str,
    size: usize,
) -> String {
    let center_str = center.unwrap_or(ccr_type);
    let meta_part = if meta.is_empty() {
        String::new()
    } else {
        format!("\n[meta:{}]", meta)
    };

    format!(
        "<<<CCR:{}|{}|{}>>>\n[{}:{}]{}",
        hash, ccr_type, size, center_str, preview, meta_part
    )
}

/// Parse the preview field from a marker line.
pub fn parse_preview(marker_line: &str) -> Option<String> {
    let start = marker_line.find('[')?;
    let colon = marker_line[start..].find(':')?;
    let end = marker_line.rfind(']')?;
    if end > start + colon {
        Some(marker_line[start + colon + 1..end].to_string())
    } else {
        None
    }
}

/// Extract all CCR hashes from text.
pub fn extract_hashes(text: &str) -> Vec<String> {
    let re = regex::Regex::new(r"(?:<<<|\[)CCR:([^|>\]\u{2af8}]+)(?:\|[^\]>]*?)?(?:\]|>>>|\u{2af8})").unwrap();
    re.captures_iter(text)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_hash() {
        assert!(is_valid_ccr_hash("abc123def456abc123def456abc123def456"));
        assert!(is_valid_ccr_hash("i:abc123def456"));
        assert!(!is_valid_ccr_hash("short"));
        assert!(!is_valid_ccr_hash(""));
    }

    #[test]
    fn test_marker_format() {
        let m = ccr_marker(
            "abc123def456abc123def456abc123def456",
            "code_rust",
            1234,
            "[code_rust:3fns 42L]",
            None, None, None,
        );
        assert!(m.contains("<<<CCR:abc123def456abc123def456abc123def456|code_rust|1234>>>"));
        assert!(m.contains("[code_rust:[code_rust:3fns 42L]"));
    }

    #[test]
    fn test_marker_with_budget() {
        let preview = "a very long preview string that should be truncated under tight budget constraints";
        let m = ccr_marker("abc123def456abc123def456abc123def456", "text", 100, preview, Some(20), None, None);
        // Budget < 25 → truncate to 30 chars
        let preview_line = m.lines().nth(1).unwrap();
        let inner = preview_line.split(':').nth(1).unwrap().trim_end_matches(']');
        assert!(inner.len() <= 32); // ~30 + bracket
    }

    #[test]
    fn test_extract_hashes() {
        let text = "<<<CCR:aaa111|code|100>>>\nsome text\n<<<CCR:bbb222|diff|200>>>";
        let hashes = extract_hashes(text);
        assert_eq!(hashes, vec!["aaa111", "bbb222"]);
    }
}
