/// Normalize a hash argument handed to a retrieve-style entry point.
///
/// LLMs sometimes echo back a whole marker body (`hash|type|size`) instead of
/// the bare hash, or wrap it in incidental whitespace (report 05 F3). Strip
/// everything from the first `|` onward and trim surrounding whitespace so
/// every retrieval site tolerates the same inputs `resolve_one` already does.
/// Idempotent: normalizing an already-bare hash is a no-op.
pub fn normalize_hash(raw:&str) -> &str { raw.split('|').next().unwrap_or(raw).trim() }

/// Check if a string is a valid CCR hash (>=24 hex chars, or `i:` prefix with
/// >=6 hex chars).
pub fn is_valid_ccr_hash(h:&str) -> bool {
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

/// Parse the preview field from a marker line.
pub fn parse_preview(marker_line:&str) -> Option<String> {
	let start = marker_line.find('[')?;
	let colon = marker_line[start..].find(':')?;
	let end = marker_line.rfind(']')?;
	if end > start + colon {
		Some(marker_line[start + colon + 1..end].to_string())
	} else {
		None
	}
}

/// Matches all three marker delimiter families this codebase (and the Python
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
static HASH_RE:std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
	regex::Regex::new(r"(?:<<<|\[|\u{2af7})CCR:([0-9a-fA-F:i]{6,64})(?:\|[^\]>\n]*?)?(?:\]|>>>|\u{2af8})").unwrap()
});

/// Extract all CCR hashes from text.
pub fn extract_hashes(text:&str) -> Vec<String> {
	HASH_RE
		.captures_iter(text)
		.filter_map(|cap| cap.get(1))
		.map(|m| m.as_str().to_string())
		.collect()
}
