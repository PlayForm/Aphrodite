//! The rendering half: the canonical grammar, the doubling-bug regression,
//! budget truncation, and the pathological-preview cases.
//!
//! Every test that renders a marker holds `Registration::Guard::Fn`: the
//! registered templates are process-global and `ConfigurationLoader`'s
//! `Apply::Startup::Fn` tests mutate them concurrently under `cargo test`.
//!
//! Each test is named for the one case it covers; the module path names the
//! subject (`Marker::Test::Format`).

use crate::Marker::{
	Format::{Entry::Fn as Marker, Registration::Guard::Fn::Fn as Serialise},
	Parse::{Extract::Fn as Extract, Preview::Fn as Read},
};

/// Component assertions rather than a full-string snapshot, so this test states
/// the actual contract: a well-formed CCR marker line followed by exactly one
/// preview line, emitted verbatim rather than re-wrapped (report 09 §5
/// doubling-bug fix).
#[test]
fn Canonical() {
	let _Serialise = Serialise();
	let Rendered = Marker(
		"abc123def456abc123def456abc123def456",
		"code_rust",
		1234,
		"[code_rust:3fns 42L]",
		None,
		None,
		None,
	);
	assert!(Rendered.contains("<<<CCR:abc123def456abc123def456abc123def456|code_rust|1234>>>"));
	// The preview text must appear in the output exactly once.
	assert_eq!(Rendered.matches("3fns 42L").count(), 1);
	// The marker line and the preview line are on separate lines.
	let mut Lines = Rendered.lines();
	assert!(Lines.next().unwrap().starts_with("<<<CCR:"));
	let Line = Lines.next().unwrap();
	assert!(Line.starts_with('['));
	assert_eq!(Line, "[code_rust:3fns 42L]");
}

// ── Doubling-bug regression (report 09 §5): a self-describing preview from
// `Preview::Builder::Entry::Fn` (e.g. `[text:53L 1913B]`) must NEVER be
// re-wrapped into `[type:[type:...]]`. The emitted marker's preview line must
// not match the `\[\w+:\[` doubling signature for ANY content type. ──

/// No content type produces a doubled bracket prefix.
#[test]
fn Doubling() {
	let _Serialise = Serialise();
	let _Preview = crate::Preview::State::Guard::Fn();
	let Pattern = regex::Regex::new(r"\[\w+:\[").unwrap();
	[
		"text",
		"git",
		"ls",
		"test",
		"grep",
		"gitlog",
		"build",
		"diff",
		"code_rust",
		"json_array",
	]
	.into_iter()
	.for_each(|Type| {
		let Preview =
			crate::Preview::Builder::Entry::Fn(Type, "M crates/aphrodite/src/preview.rs\nA src/new.rs\n?? tmp\n");
		let Rendered = Marker("abc123def456abc123def456abc123def456", Type, 42, &Preview, None, None, None);
		assert!(
			!Pattern.is_match(&Rendered),
			"marker preview must not double the bracket prefix for type {Type:?}: {Rendered:?}"
		);
	});
}

/// A bare (cache-mode) excerpt is wrapped once with the center label; an
/// already-bracketed preview is emitted verbatim.
#[test]
fn Wrapping() {
	let _Serialise = Serialise();
	let Bare = Marker(
		"abc123def456abc123def456abc123def456",
		"text",
		5,
		"hello world",
		None,
		None,
		None,
	);
	assert!(Bare.contains("\n[text:hello world]"));

	let Bracketed = Marker(
		"abc123def456abc123def456abc123def456",
		"git",
		5,
		"[git:2M | a.rs]",
		None,
		None,
		None,
	);
	assert!(Bracketed.contains("\n[git:2M | a.rs]"));
	assert!(!Bracketed.contains("[git:[git:"));
}

/// A budget under 25 truncates to 30 characters.
#[test]
fn Truncation() {
	let _Serialise = Serialise();
	let Preview = "a very long preview string that should be truncated under tight budget constraints";
	let Rendered = Marker(
		"abc123def456abc123def456abc123def456",
		"text",
		100,
		Preview,
		Some(20),
		None,
		None,
	);
	let Line = Rendered.lines().nth(1).unwrap();
	let Inner = Line.split(':').nth(1).unwrap().trim_end_matches(']');
	assert!(Inner.len() <= 32); // ~30 + bracket
}

// ── 04-T9: pathological-input coverage (UTF-8 boundary, literal
// markers, interior NUL) for the marker module. ──

/// The control-char filter excludes NUL (0x00) along with every other C0
/// control code - a literal embedded NUL in tool output must not survive into
/// the rendered marker.
#[test]
fn Nul() {
	let _Serialise = Serialise();
	let Preview = "before\0after";
	let Rendered = Marker("abc123def456abc123def456abc123def456", "text", 12, Preview, None, None, None);
	assert!(
		!Rendered.contains('\0'),
		"NUL byte must be stripped from the preview: {Rendered:?}"
	);
	assert!(Rendered.contains("beforeafter"));
}

/// A budget under 25 truncates to 30 *chars* rather than 30 bytes, so a preview
/// packed with multi-byte UTF-8 (each `é` is 2 bytes) never splits a character.
#[test]
fn Multibyte() {
	let _Serialise = Serialise();
	let Preview = "é".repeat(50);
	let Rendered = Marker(
		"abc123def456abc123def456abc123def456",
		"text",
		100,
		&Preview,
		Some(10), // budget < 25 -> 30 chars
		None,
		None,
	);
	let Line = Rendered.lines().nth(1).unwrap();
	let Inner = Read(Line).unwrap();
	assert_eq!(Inner.chars().count(), 30, "must truncate to exactly 30 chars, not 30 bytes");
}

/// Literal marker-shaped text in the preview survives verbatim, and only the
/// real hash is extracted: the real hash is always on line 1, and `|`-mangling
/// would harm every enriched preview (ls, git, test, grep all use `|` as a
/// visual separator) to defend against a vanishingly rare edge case.
#[test]
fn Literal() {
	let _Serialise = Serialise();
	let Text = "example: <<<CCR:fake000|text|1>>>";
	let Rendered = Marker("abc123def456abc123def456abc123def456", "text", 999, Text, None, None, None);
	assert!(Rendered.starts_with("<<<CCR:abc123def456abc123def456abc123def456|text|999>>>"));
	// `fake000` contains `k`, outside `[0-9a-fA-F]`, so the hash gate already
	// rejects the embedded literal - only the real hash is extracted.
	assert!(
		Rendered.contains("fake000|text|1"),
		"embedded literal text survives verbatim (pipes intact): {Rendered:?}"
	);
	let Hashes = Extract(&Rendered);
	assert_eq!(Hashes, vec!["abc123def456abc123def456abc123def456"]);
}
