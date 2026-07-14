//! CCR marker generation - 1:1 port of plugins/aphrodite/_marker/marker.py
//!
//! Generates <<<CCR:hash|type|size>>> markers with TOML-driven templates.

use std::collections::HashMap;

/// Normalize a hash argument handed to a retrieve-style entry point.
///
/// LLMs sometimes echo back a whole marker body (`hash|type|size`) instead of
/// the bare hash, or wrap it in incidental whitespace (report 05 F3). Strip
/// everything from the first `|` onward and trim surrounding whitespace so
/// every retrieval site tolerates the same inputs `resolve_one` already does.
/// Idempotent: normalizing an already-bare hash is a no-op.
pub fn normalize_hash(raw: &str) -> &str {
	raw.split('|').next().unwrap_or(raw).trim()
}

/// Check if a string is a valid CCR hash (>=24 hex chars, or `i:` prefix with
/// >=6 hex chars).
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
///
/// - `hash_val`: the content hash
/// - `ccr_type`: content type string (e.g. "code_rust", "build")
/// - `size`: original content size in bytes
/// - `preview`: the formatted preview string
/// - `headroom_budget`: optional token budget for truncation
/// - `meta`: optional metadata key-value pairs
/// - `center`: optional center annotation
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
	let mut safe = preview.replace('|', "-").replace(['\n', '\r'], " ").trim().to_string();
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
			s = format!("{}...", crate::struct_extract::floor_boundary(&s, 297));
		}
		s
	} else {
		String::new()
	};

	// Build marker using the standard template
	render_marker(&safe, ccr_type, &meta_str, center, hash_val, size)
}

/// Render the marker using the canonical three-line format.
fn render_marker(preview: &str, ccr_type: &str, meta: &str, center: Option<&str>, hash: &str, size: usize) -> String {
	let center_str = center.unwrap_or(ccr_type);
	let meta_part = if meta.is_empty() { String::new() } else { format!("\n[meta:{}]", meta) };

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

/// Matches all four marker delimiter families this codebase (and the Python
/// plugin / docs) uses to wrap a CCR reference:
/// `<<<CCR:hash|type|size>>>`, `[CCR:hash|type]`, and the Unicode-glyph forms
/// opened by `⫷` (U+2AF7) or closed by `⫸` (U+2AF8). Compiled once (report 05
/// F7: the previous per-call `Regex::new(...).unwrap()` both recompiled the
/// pattern on every call and could panic on a bad literal - a `LazyLock`
/// makes the "never fails" invariant of a hardcoded pattern checked exactly
/// once, at first use, instead of on every call).
///
/// The hash class is anchored to `[0-9a-fA-F:i]{6,64}` - hex digits, or the
/// `i:` inline-hash prefix followed by hex - rather than "anything that
/// isn't a delimiter", which previously let the capture cross a newline
/// (`<<<CCR:` on one line, `>>>` several lines later, and everything
/// between - including other markers - matched as one "hash"). `\n` is also
/// excluded from the trailing metadata segment for the same reason. Missing
/// the `⫷` opener (previously accepted `⫸` as a closer but never `⫷` as an
/// opener) meant the Unicode-glyph marker style was silently never
/// extracted at all.
static HASH_RE: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
	regex::Regex::new(r"(?:<<<|\[|\u{2af7})CCR:([0-9a-fA-F:i]{6,64})(?:\|[^\]>\n]*?)?(?:\]|>>>|\u{2af8})").unwrap()
});

/// Extract all CCR hashes from text.
pub fn extract_hashes(text: &str) -> Vec<String> {
	HASH_RE
		.captures_iter(text)
		.filter_map(|cap| cap.get(1))
		.map(|m| m.as_str().to_string())
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	// ── T5 (F3): normalize_hash ────────────────────────────────────
	#[test]
	fn test_normalize_hash_bare_is_unchanged() {
		assert_eq!(normalize_hash("abc123"), "abc123");
	}

	#[test]
	fn test_normalize_hash_strips_pipe_suffix() {
		assert_eq!(normalize_hash("abc123|tool|1024"), "abc123");
	}

	#[test]
	fn test_normalize_hash_trims_whitespace() {
		assert_eq!(normalize_hash("  abc123  "), "abc123");
	}

	#[test]
	fn test_normalize_hash_is_idempotent() {
		let once = normalize_hash("  abc123|tool|1024  ");
		assert_eq!(normalize_hash(once), once);
	}

	#[test]
	fn test_valid_hash() {
		assert!(is_valid_ccr_hash("abc123def456abc123def456abc123def456"));
		assert!(is_valid_ccr_hash("i:abc123def456"));
		assert!(!is_valid_ccr_hash("short"));
		assert!(!is_valid_ccr_hash(""));
	}

	#[test]
	fn test_marker_format() {
		// Component assertions rather than a full-string snapshot, so this
		// test states the actual contract (a well-formed CCR marker line
		// followed by exactly one preview line) instead of pinning a specific
		// rendering as "correct". NOTE: `render_marker` currently prints
		// `[{ct}:{preview}]`, and when `preview` already carries its own
		// `[{ct}:...]` prefix (as built by `build_preview`), the result
		// visibly doubles the type tag (see .plans/09-testing-quality.md §5
		// "doubled marker prefix" - open question for the user, not changed
		// here).
		let m = ccr_marker(
			"abc123def456abc123def456abc123def456",
			"code_rust",
			1234,
			"[code_rust:3fns 42L]",
			None,
			None,
			None,
		);
		assert!(m.contains("<<<CCR:abc123def456abc123def456abc123def456|code_rust|1234>>>"));
		// The preview text must appear in the output exactly once.
		assert_eq!(m.matches("3fns 42L").count(), 1);
		// The marker line and the preview line are on separate lines.
		let mut lines = m.lines();
		assert!(lines.next().unwrap().starts_with("<<<CCR:"));
		assert!(lines.next().unwrap().starts_with('['));
	}

	#[test]
	fn test_marker_with_budget() {
		let preview = "a very long preview string that should be truncated under tight budget constraints";
		let m = ccr_marker(
			"abc123def456abc123def456abc123def456",
			"text",
			100,
			preview,
			Some(20),
			None,
			None,
		);
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

	// ── T7: is_valid_ccr_hash boundary band ──────────────────────
	#[test]
	fn test_is_valid_ccr_hash_boundary_band() {
		// 8..23 hex chars: below the 24-char full-hash floor -> false.
		assert!(!is_valid_ccr_hash("abcdef12")); // 8 hex chars
		assert!(!is_valid_ccr_hash("abcdef0123456789abcdef")); // 22 hex chars
		assert!(!is_valid_ccr_hash("abcdef0123456789abcdeff")); // 23 hex chars
		// 24 hex chars: at the floor -> true.
		assert!(is_valid_ccr_hash("abcdef0123456789abcdef01")); // 24 hex chars
	}

	#[test]
	fn test_is_valid_ccr_hash_i_prefix_variants() {
		assert!(!is_valid_ccr_hash("i:xyz")); // not hex, too short
		assert!(is_valid_ccr_hash("i:abc123")); // 6 hex chars after i:
		assert!(!is_valid_ccr_hash("i:abc1")); // only 4 hex chars after i:
	}

	#[test]
	fn test_is_valid_ccr_hash_uppercase() {
		assert!(is_valid_ccr_hash("ABCDEF0123456789ABCDEF01"));
	}

	// ── T7: extract_hashes delimiter families ────────────────────
	#[test]
	fn test_extract_hashes_bracket_form() {
		let hashes = extract_hashes("[CCR:aaa111|code]");
		assert_eq!(hashes, vec!["aaa111"]);
	}

	#[test]
	fn test_extract_hashes_glyph_terminated_form() {
		// extract_hashes's regex accepts the glyph terminator \u{2af8} as an
		// alternative to `]`/`>>>`; the hash capture stops at the first `|`,
		// same as the `<<<CCR:...>>>` and `[CCR:...]` forms.
		let text = "<<<CCR:ccc333|text\u{2af8}";
		let hashes = extract_hashes(text);
		assert_eq!(hashes, vec!["ccc333"]);
	}

	#[test]
	fn test_extract_hashes_unterminated_no_match() {
		assert!(extract_hashes("<<<CCR:no_terminator_here").is_empty());
	}

	// ── T7 (F7): the `⫷` (U+2AF7) opening glyph was previously never
	// recognized - only its `⫸` (U+2AF8) closing counterpart was - so the
	// Unicode-glyph marker style was silently never extracted at all.
	#[test]
	fn test_extract_hashes_full_glyph_delimited_form() {
		let hashes = extract_hashes("\u{2af7}CCR:abc123\u{2af8}");
		assert_eq!(hashes, vec!["abc123"]);
	}

	// ── T7 (F7): the old hash class `[^|>\]\u{2af8}]+` matched across
	// newlines, so an unclosed `<<<CCR:` on one line and a `>>>` several
	// lines later (possibly past other, unrelated markers) would be
	// captured as one garbage "hash". The hash class is now anchored to
	// hex/`i:` characters only, which can never include a newline.
	#[test]
	fn test_extract_hashes_does_not_cross_newlines() {
		let text = "<<<CCR:foo\nbar>>>";
		assert!(extract_hashes(text).is_empty(), "must not capture a multi-line garbage hash");
	}

	#[test]
	fn test_parse_preview_on_garbage() {
		assert_eq!(parse_preview("no brackets here"), None);
		assert_eq!(parse_preview("[nocolon]"), None);
		assert_eq!(parse_preview("[code_rust:hello]"), Some("hello".to_string()));
	}
}
