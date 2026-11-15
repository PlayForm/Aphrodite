//! The rendering half: the canonical grammar, the doubling-bug regression,
//! budget truncation, and the pathological-preview cases.
//!
//! Every test that renders a marker holds `marker_templates_test_guard()`: the
//! registered templates are process-global and `ConfigurationLoader`'s
//! `ApplyCompression` tests mutate them concurrently under `cargo test`.

use crate::Marker::{
	Format::{Const::MarkerTemplatesTestGuard, Entry::CCRMarker},
	Parse::{Extract::ExtractHashes, Preview::ParsePreview},
};

#[test]
fn TestMarkerFormat() {
	// Component assertions rather than a full-string snapshot, so this
	// test states the actual contract (a well-formed CCR marker line
	// followed by exactly one preview line). `BuildPreview` returns a
	// self-describing `[ct:...]` preview; `render_marker` must emit it
	// verbatim (report 09 §5 doubling-bug fix) rather than re-wrapping it
	// into `[ct:[ct:...]]`.
	//
	// All tests in this module that build markers hold
	// `marker_templates_test_guard()`: the registered templates are
	// process-global and config_loader's `ApplyCompression` tests mutate
	// them concurrently under `cargo test`.
	let _G = MarkerTemplatesTestGuard();
	let m = CCRMarker(
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
	let mut Lines = m.lines();
	assert!(Lines.next().unwrap().starts_with("<<<CCR:"));
	let PreviewLine = Lines.next().unwrap();
	assert!(PreviewLine.starts_with('['));
	// Doubling-bug fix: the preview line is the self-describing preview
	// verbatim, NOT re-wrapped into `[code_rust:[code_rust:...]]`.
	assert_eq!(PreviewLine, "[code_rust:3fns 42L]");
}

// ── Doubling-bug regression (report 09 §5): a self-describing preview from
// `BuildPreview` (e.g. `[text:53L 1913B]`) must NEVER be re-wrapped into
// `[type:[type:...]]`. The emitted marker's preview line must not match the
// `\[\w+:\[` doubling signature for ANY content type. ──
#[test]
fn TestMarkerPreviewNeverDoublesBracketPrefix() {
	let _G = crate::Preview::State::Guard::Fn();
	let _G2 = MarkerTemplatesTestGuard();
	let Re = regex::Regex::new(r"\[\w+:\[").unwrap();
	for ty in [
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
	] {
		let Preview =
			crate::Preview::Builder::Entry::Fn(ty, "M crates/aphrodite/src/preview.rs\nA src/new.rs\n?? tmp\n");
		let m = CCRMarker("abc123def456abc123def456abc123def456", ty, 42, &Preview, None, None, None);
		assert!(
			!Re.is_match(&m),
			"marker preview must not double the bracket prefix for type {ty:?}: {m:?}"
		);
	}
}

#[test]
fn TestRenderMarkerWrapsBarePreviewButNotBracketed() {
	let _G = MarkerTemplatesTestGuard();
	// A bare (cache-mode) excerpt is wrapped once with the center label...
	let Bare = CCRMarker(
		"abc123def456abc123def456abc123def456",
		"text",
		5,
		"hello world",
		None,
		None,
		None,
	);
	assert!(Bare.contains("\n[text:hello world]"));
	// ...but an already-bracketed preview is emitted verbatim.
	let Bracketed = CCRMarker(
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

#[test]
fn TestMarkerWithBudget() {
	let _G = MarkerTemplatesTestGuard();
	let Preview = "a very long preview string that should be truncated under tight budget constraints";
	let m = CCRMarker(
		"abc123def456abc123def456abc123def456",
		"text",
		100,
		Preview,
		Some(20),
		None,
		None,
	);
	// Budget < 25 → truncate to 30 chars
	let PreviewLine = m.lines().nth(1).unwrap();
	let Inner = PreviewLine.split(':').nth(1).unwrap().trim_end_matches(']');
	assert!(Inner.len() <= 32); // ~30 + bracket
}

// ── 04-T9: pathological-input coverage (UTF-8 boundary, literal
// markers, interior NUL) for the marker module. ──

#[test]
fn TestCCRMarkerStripsInteriorNulBytes() {
	let _G = MarkerTemplatesTestGuard();
	// The control-char filter (`*c >= ' '`) excludes NUL (0x00) along with
	// every other C0 control code - a literal embedded NUL in tool output
	// must not survive into the rendered marker.
	let Preview = "before\0after";
	let m = CCRMarker("abc123def456abc123def456abc123def456", "text", 12, Preview, None, None, None);
	assert!(!m.contains('\0'), "NUL byte must be stripped from the preview: {m:?}");
	assert!(m.contains("beforeafter"));
}

#[test]
fn TestCCRMarkerTruncatesMultibytePreviewOnCharBoundary() {
	let _G = MarkerTemplatesTestGuard();
	// budget < 25 truncates to 30 *chars* via `.chars().take(n)`, not a
	// byte slice - a preview packed with multi-byte UTF-8 (each 'é' is 2
	// bytes) must not panic or produce a str that isn't valid UTF-8 when
	// the char boundary and a naive byte boundary would disagree.
	let Preview = "é".repeat(50);
	let m = CCRMarker(
		"abc123def456abc123def456abc123def456",
		"text",
		100,
		&Preview,
		Some(10), // budget < 25 -> take(30) chars
		None,
		None,
	);
	let PreviewLine = m.lines().nth(1).unwrap();
	let Inner = ParsePreview(PreviewLine).unwrap();
	assert_eq!(Inner.chars().count(), 30, "must truncate to exactly 30 chars, not 30 bytes");
}

#[test]
fn TestCCRMarkerPreviewContainingLiteralMarkerSyntaxDoesNotConfuseExtraction() {
	let _G = MarkerTemplatesTestGuard();
	// Literal marker-shaped text in the preview: `ExtractHashes` scans the
	// FULL output (including the preview line below the marker). Without `|`
	// sanitization, a literal `<<<CCR:hex|type|size>>>` in content will
	// survive intact, so `ExtractHashes` would also match the literal token
	// alongside the real marker hash. This is acceptable: the real hash is
	// always first (line 1), and `|`-mangling harmed every enriched preview
	// (ls, git, test, grep - all use `|` as a visual separator in their
	// format) to defend against a vanishingly rare edge case (tool output
	// literally containing `<<<CCR:`-shaped text).
	let LiteralMarkerText = "example: <<<CCR:fake000|text|1>>>";
	let m = CCRMarker(
		"abc123def456abc123def456abc123def456",
		"text",
		999,
		LiteralMarkerText,
		None,
		None,
		None,
	);
	assert!(m.starts_with("<<<CCR:abc123def456abc123def456abc123def456|text|999>>>"));
	// Without | → `-` sanitization, the literal `<<<CCR:fake000|text|1>>>`
	// in the preview survives intact - but `fake000` contains `k` which is
	// outside [a-f], so `ExtractHashes`'s hex-hash gate `[0-9a-fA-F]`
	// already rejects it. Only the real hash (pure hex) is extracted.
	assert!(
		m.contains("fake000|text|1"),
		"embedded literal text survives verbatim (pipes intact): {m:?}"
	);
	let Hashes = ExtractHashes(&m);
	assert_eq!(Hashes, vec!["abc123def456abc123def456abc123def456"]);
}
