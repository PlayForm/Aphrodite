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
	// followed by exactly one preview line). `build_preview` returns a
	// self-describing `[ct:...]` preview; `render_marker` must emit it
	// verbatim (report 09 §5 doubling-bug fix) rather than re-wrapping it
	// into `[ct:[ct:...]]`.
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
	let preview_line = lines.next().unwrap();
	assert!(preview_line.starts_with('['));
	// Doubling-bug fix: the preview line is the self-describing preview
	// verbatim, NOT re-wrapped into `[code_rust:[code_rust:...]]`.
	assert_eq!(preview_line, "[code_rust:3fns 42L]");
}

// ── Doubling-bug regression (report 09 §5): a self-describing preview from
// `build_preview` (e.g. `[text:53L 1913B]`) must NEVER be re-wrapped into
// `[type:[type:...]]`. The emitted marker's preview line must not match the
// `\[\w+:\[` doubling signature for ANY content type. ──
#[test]
fn test_marker_preview_never_doubles_bracket_prefix() {
	let _g = crate::preview::preview_cap_test_guard();
	let re = regex::Regex::new(r"\[\w+:\[").unwrap();
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
		let preview = crate::build_preview(ty, "M crates/aphrodite/src/preview.rs\nA src/new.rs\n?? tmp\n");
		let m = ccr_marker("abc123def456abc123def456abc123def456", ty, 42, &preview, None, None, None);
		assert!(
			!re.is_match(&m),
			"marker preview must not double the bracket prefix for type {ty:?}: {m:?}"
		);
	}
}

#[test]
fn test_render_marker_wraps_bare_preview_but_not_bracketed() {
	// A bare (cache-mode) excerpt is wrapped once with the center label...
	let bare = ccr_marker(
		"abc123def456abc123def456abc123def456",
		"text",
		5,
		"hello world",
		None,
		None,
		None,
	);
	assert!(bare.contains("\n[text:hello world]"));
	// ...but an already-bracketed preview is emitted verbatim.
	let bracketed = ccr_marker(
		"abc123def456abc123def456abc123def456",
		"git",
		5,
		"[git:2M | a.rs]",
		None,
		None,
		None,
	);
	assert!(bracketed.contains("\n[git:2M | a.rs]"));
	assert!(!bracketed.contains("[git:[git:"));
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

// ── 04-T9: pathological-input coverage (UTF-8 boundary, literal
// markers, interior NUL) for the marker module. ──

#[test]
fn test_ccr_marker_strips_interior_nul_bytes() {
	// The control-char filter (`*c >= ' '`) excludes NUL (0x00) along with
	// every other C0 control code - a literal embedded NUL in tool output
	// must not survive into the rendered marker.
	let preview = "before\0after";
	let m = ccr_marker("abc123def456abc123def456abc123def456", "text", 12, preview, None, None, None);
	assert!(!m.contains('\0'), "NUL byte must be stripped from the preview: {m:?}");
	assert!(m.contains("beforeafter"));
}

#[test]
fn test_ccr_marker_truncates_multibyte_preview_on_char_boundary() {
	// budget < 25 truncates to 30 *chars* via `.chars().take(n)`, not a
	// byte slice - a preview packed with multi-byte UTF-8 (each 'é' is 2
	// bytes) must not panic or produce a str that isn't valid UTF-8 when
	// the char boundary and a naive byte boundary would disagree.
	let preview = "é".repeat(50);
	let m = ccr_marker(
		"abc123def456abc123def456abc123def456",
		"text",
		100,
		&preview,
		Some(10), // budget < 25 -> take(30) chars
		None,
		None,
	);
	let preview_line = m.lines().nth(1).unwrap();
	let inner = parse_preview(preview_line).unwrap();
	assert_eq!(inner.chars().count(), 30, "must truncate to exactly 30 chars, not 30 bytes");
}

#[test]
fn test_ccr_marker_preview_containing_literal_marker_syntax_does_not_confuse_extraction() {
	// Literal marker-shaped text in the preview: `extract_hashes` scans the
	// FULL output (including the preview line below the marker). Without `|`
	// sanitization, a literal `<<<CCR:hex|type|size>>>` in content will
	// survive intact, so `extract_hashes` would also match the literal token
	// alongside the real marker hash. This is acceptable: the real hash is
	// always first (line 1), and `|`-mangling harmed every enriched preview
	// (ls, git, test, grep - all use `|` as a visual separator in their
	// format) to defend against a vanishingly rare edge case (tool output
	// literally containing `<<<CCR:`-shaped text).
	let literal_marker_text = "example: <<<CCR:fake000|text|1>>>";
	let m = ccr_marker(
		"abc123def456abc123def456abc123def456",
		"text",
		999,
		literal_marker_text,
		None,
		None,
		None,
	);
	assert!(m.starts_with("<<<CCR:abc123def456abc123def456abc123def456|text|999>>>"));
	// Without | → `-` sanitization, the literal `<<<CCR:fake000|text|1>>>`
	// in the preview survives intact - but `fake000` contains `k` which is
	// outside [a-f], so `extract_hashes`'s hex-hash gate `[0-9a-fA-F]`
	// already rejects it. Only the real hash (pure hex) is extracted.
	assert!(
		m.contains("fake000|text|1"),
		"embedded literal text survives verbatim (pipes intact): {m:?}"
	);
	let hashes = extract_hashes(&m);
	assert_eq!(hashes, vec!["abc123def456abc123def456abc123def456"]);
}
