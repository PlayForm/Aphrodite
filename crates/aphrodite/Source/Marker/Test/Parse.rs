//! The reading half: hash normalization, hash validity, hash extraction across
//! all three delimiter families, and preview parsing.
//!
//! Each test is named for the one case it covers; the module path names the
//! subject (`Marker::Test::Parse`).

use crate::Marker::Parse::{
	Extract::Fn as Extract,
	Normalize::Fn as Normalize,
	Preview::Fn as Preview,
	Valid::Fn as Valid,
};

// ── T5 (F3): hash normalization ───────────────────────────────

/// An already-bare hash is answered unchanged.
#[test]
fn Bare() {
	assert_eq!(Normalize("abc123"), "abc123");
}

/// Everything from the first `|` onward is dropped.
#[test]
fn Pipe() {
	assert_eq!(Normalize("abc123|tool|1024"), "abc123");
}

/// Surrounding whitespace is trimmed.
#[test]
fn Whitespace() {
	assert_eq!(Normalize("  abc123  "), "abc123");
}

/// Normalizing an already-normalized hash changes nothing.
#[test]
fn Idempotent() {
	let Once = Normalize("  abc123|tool|1024  ");
	assert_eq!(Normalize(Once), Once);
}

/// Both valid shapes pass and short or empty strings do not.
#[test]
fn Validity() {
	assert!(Valid("abc123def456abc123def456abc123def456"));
	assert!(Valid("i:abc123def456"));
	assert!(!Valid("short"));
	assert!(!Valid(""));
}

/// Two markers in one blob are extracted in order.
#[test]
fn Pairs() {
	let Text = "<<<CCR:aaa111|code|100>>>\nsome text\n<<<CCR:bbb222|diff|200>>>";
	let Hashes = Extract(Text);
	assert_eq!(Hashes, vec!["aaa111", "bbb222"]);
}

// ── T7: the full-hash length boundary ────────────────────────

/// 8..23 hex chars sit below the 24-char full-hash floor; 24 is the floor.
#[test]
fn Boundary() {
	assert!(!Valid("abcdef12")); // 8 hex chars
	assert!(!Valid("abcdef0123456789abcdef")); // 22 hex chars
	assert!(!Valid("abcdef0123456789abcdeff")); // 23 hex chars
	assert!(Valid("abcdef0123456789abcdef01")); // 24 hex chars
}

/// The `i:` inline prefix needs six hex characters after it.
#[test]
fn Prefix() {
	assert!(!Valid("i:xyz")); // not hex, too short
	assert!(Valid("i:abc123")); // 6 hex chars after i:
	assert!(!Valid("i:abc1")); // only 4 hex chars after i:
}

/// Uppercase hex is valid - the check never lowercases.
#[test]
fn Uppercase() {
	assert!(Valid("ABCDEF0123456789ABCDEF01"));
}

// ── T7: extraction across the delimiter families ─────────────

/// The `[CCR:hash|type]` family.
#[test]
fn Bracket() {
	let Hashes = Extract("[CCR:aaa111|code]");
	assert_eq!(Hashes, vec!["aaa111"]);
}

/// The glyph terminator `⫸` (U+2AF8) closes a `<<<CCR:` form; the hash capture
/// stops at the first `|`, same as the `<<<CCR:...>>>` and `[CCR:...]` forms.
#[test]
fn Glyph() {
	let Text = "<<<CCR:ccc333|text\u{2af8}";
	let Hashes = Extract(Text);
	assert_eq!(Hashes, vec!["ccc333"]);
}

/// An unterminated marker matches nothing.
#[test]
fn Unterminated() {
	assert!(Extract("<<<CCR:no_terminator_here").is_empty());
}

// ── T7 (F7): the `⫷` (U+2AF7) opening glyph was previously never
// recognized - only its `⫸` (U+2AF8) closing counterpart was - so the
// Unicode-glyph marker style was silently never extracted at all.

/// Both glyph delimiters together.
#[test]
fn Delimited() {
	let Hashes = Extract("\u{2af7}CCR:abc123\u{2af8}");
	assert_eq!(Hashes, vec!["abc123"]);
}

// ── T7 (F7): the old hash class `[^|>\]\u{2af8}]+` matched across
// newlines, so an unclosed `<<<CCR:` on one line and a `>>>` several
// lines later (possibly past other, unrelated markers) would be
// captured as one garbage "hash". The hash class is now anchored to
// hex/`i:` characters only, which can never include a newline.

/// A capture never spans a newline.
#[test]
fn Newline() {
	let Text = "<<<CCR:foo\nbar>>>";
	assert!(Extract(Text).is_empty(), "must not capture a multi-line garbage hash");
}

/// A line that is not preview-shaped is `None`; a preview-shaped one answers its
/// inner text.
#[test]
fn Garbage() {
	assert_eq!(Preview("no brackets here"), None);
	assert_eq!(Preview("[nocolon]"), None);
	assert_eq!(Preview("[code_rust:hello]"), Some("hello".to_string()));
}
