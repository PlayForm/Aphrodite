//! The reading half: hash normalization, hash validity, hash extraction across
//! all three delimiter families, and preview parsing.

use crate::Marker::Parse::Extract::ExtractHashes;
use crate::Marker::Parse::Valid::IsValidCCRHash;
use crate::Marker::Parse::Normalize::NormalizeHash;
use crate::Marker::Parse::Preview::ParsePreview;

// ── T5 (F3): normalize_hash ────────────────────────────────────
#[test]
fn TestNormalizeHashBareIsUnchanged() {
	assert_eq!(NormalizeHash("abc123"), "abc123");
}

#[test]
fn TestNormalizeHashStripsPipeSuffix() {
	assert_eq!(NormalizeHash("abc123|tool|1024"), "abc123");
}

#[test]
fn TestNormalizeHashTrimsWhitespace() {
	assert_eq!(NormalizeHash("  abc123  "), "abc123");
}

#[test]
fn TestNormalizeHashIsIdempotent() {
	let Once = NormalizeHash("  abc123|tool|1024  ");
	assert_eq!(NormalizeHash(Once), Once);
}

#[test]
fn TestValidHash() {
	assert!(IsValidCCRHash("abc123def456abc123def456abc123def456"));
	assert!(IsValidCCRHash("i:abc123def456"));
	assert!(!IsValidCCRHash("short"));
	assert!(!IsValidCCRHash(""));
}

#[test]
fn TestExtractHashes() {
	let Text = "<<<CCR:aaa111|code|100>>>\nsome text\n<<<CCR:bbb222|diff|200>>>";
	let Hashes = ExtractHashes(Text);
	assert_eq!(Hashes, vec!["aaa111", "bbb222"]);
}

// ── T7: is_valid_ccr_hash boundary band ──────────────────────
#[test]
fn TestIsValidCCRHashBoundaryBand() {
	// 8..23 hex chars: below the 24-char full-hash floor -> false.
	assert!(!IsValidCCRHash("abcdef12")); // 8 hex chars
	assert!(!IsValidCCRHash("abcdef0123456789abcdef")); // 22 hex chars
	assert!(!IsValidCCRHash("abcdef0123456789abcdeff")); // 23 hex chars
	// 24 hex chars: at the floor -> true.
	assert!(IsValidCCRHash("abcdef0123456789abcdef01")); // 24 hex chars
}

#[test]
fn TestIsValidCCRHashIPrefixVariants() {
	assert!(!IsValidCCRHash("i:xyz")); // not hex, too short
	assert!(IsValidCCRHash("i:abc123")); // 6 hex chars after i:
	assert!(!IsValidCCRHash("i:abc1")); // only 4 hex chars after i:
}

#[test]
fn TestIsValidCCRHashUppercase() {
	assert!(IsValidCCRHash("ABCDEF0123456789ABCDEF01"));
}

// ── T7: extract_hashes delimiter families ────────────────────
#[test]
fn TestExtractHashesBracketForm() {
	let Hashes = ExtractHashes("[CCR:aaa111|code]");
	assert_eq!(Hashes, vec!["aaa111"]);
}

#[test]
fn TestExtractHashesGlyphTerminatedForm() {
	// extract_hashes's regex accepts the glyph terminator \u{2af8} as an
	// alternative to `]`/`>>>`; the hash capture stops at the first `|`,
	// same as the `<<<CCR:...>>>` and `[CCR:...]` forms.
	let Text = "<<<CCR:ccc333|text\u{2af8}";
	let Hashes = ExtractHashes(Text);
	assert_eq!(Hashes, vec!["ccc333"]);
}

#[test]
fn TestExtractHashesUnterminatedNoMatch() {
	assert!(ExtractHashes("<<<CCR:no_terminator_here").is_empty());
}

// ── T7 (F7): the `⫷` (U+2AF7) opening glyph was previously never
// recognized - only its `⫸` (U+2AF8) closing counterpart was - so the
// Unicode-glyph marker style was silently never extracted at all.
#[test]
fn TestExtractHashesFullGlyphDelimitedForm() {
	let Hashes = ExtractHashes("\u{2af7}CCR:abc123\u{2af8}");
	assert_eq!(Hashes, vec!["abc123"]);
}

// ── T7 (F7): the old hash class `[^|>\]\u{2af8}]+` matched across
// newlines, so an unclosed `<<<CCR:` on one line and a `>>>` several
// lines later (possibly past other, unrelated markers) would be
// captured as one garbage "hash". The hash class is now anchored to
// hex/`i:` characters only, which can never include a newline.
#[test]
fn TestExtractHashesDoesNotCrossNewlines() {
	let Text = "<<<CCR:foo\nbar>>>";
	assert!(ExtractHashes(Text).is_empty(), "must not capture a multi-line garbage hash");
}

#[test]
fn TestParsePreviewOnGarbage() {
	assert_eq!(ParsePreview("no brackets here"), None);
	assert_eq!(ParsePreview("[nocolon]"), None);
	assert_eq!(ParsePreview("[code_rust:hello]"), Some("hello".to_string()));
}
