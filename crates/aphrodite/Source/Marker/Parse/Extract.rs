//! Extracting every CCR hash from arbitrary text.
//!
//! Three delimiter families are accepted because three are in circulation:
//! `<<<CCR:hash|type|size>>>`, `[CCR:hash|type]`, and the Unicode-glyph form
//! opened by `⫷` (U+2AF7) or closed by `⫸` (U+2AF8). The hash class is
//! anchored to `[0-9a-fA-F:i]{6,64}` - hex digits, or the `i:` inline-hash
//! prefix followed by hex - rather than "anything that isn't a delimiter",
//! which previously let the capture cross a newline (`<<<CCR:` on one line,
//! `>>>` several lines later, and everything between - including other
//! markers - matched as one "hash"). `\n` is excluded from the trailing
//! metadata segment for the same reason.
//!
//! Compiled once: the previous per-call `Regex::new(...).unwrap()` recompiled
//! the pattern on every call and could panic on a bad literal - a `LazyLock`
//! makes the "never fails" invariant of a hardcoded pattern checked exactly
//! once, at first use, instead of on every call (report 05 F7). Missing the
//! `⫷` opener (previously accepted `⫸` as a closer but never `⫷` as an opener)
//! meant the Unicode-glyph marker style was silently never extracted at all.

static HashRe:std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
	regex::Regex::new(r"(?:<<<|\[|\u{2af7})CCR:([0-9a-fA-F:i]{6,64})(?:\|[^\]>\n]*?)?(?:\]|>>>|\u{2af8})").unwrap()
});

/// Extract all CCR hashes from text.
pub fn ExtractHashes(Text:&str) -> Vec<String> {
	HashRe
		.captures_iter(Text)
		.filter_map(|Cap| Cap.get(1))
		.map(|m| m.as_str().to_string())
		.collect()
}
