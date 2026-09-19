//! Preview test battery - the frozen contract (1.5.0: moved verbatim from the
//! old monolithic `preview.rs` into its own file; see REFACTOR-PLAN-1.5.0).

use super::*;

// ── 04-T9: pathological-input coverage (UTF-8 boundary, literal
// markers, interior NUL) - previously untested in this module. ──

#[test]
fn test_detect_type_never_panics_on_interior_nul() {
	let _g = cap_guard();
	let content = "before\0after\0\0end";
	let _ = detect_type(content); // must not panic
}

#[test]
fn test_build_preview_never_panics_on_interior_nul_across_type_branches() {
	let _g = cap_guard();
	let content = "line one\0line two\0\0error: boom\nwarning: also this";
	for ty in [
		"build",
		"diff",
		"code_rust",
		"code_sh",
		"code",
		"search",
		"html",
		"json_array",
		"json",
		"json_list",
		"terminal",
		"text",
	] {
		let _ = build_preview(ty, content); // must not panic for any branch
	}
}

#[test]
fn test_build_preview_code_rust_truncates_signature_on_char_boundary() {
	let _g = cap_guard();
	// The "code" family truncates the first signature to 48 *chars* via
	// `.chars().take(48)`, not a byte slice - a signature packed with
	// multi-byte UTF-8 must not panic or split a character mid-encoding.
	// This function never panics on reaching an assertion at all (a byte-
	// boundary split would have panicked inside `String` construction
	// before we got here), so the pass condition is simply completing
	// without a panic and producing the expected preview shape.
	let content = format!("fn \u{4e2d}\u{6587}_{}() {{}}", "x".repeat(60));
	let out = build_preview("code_rust", &content);
	assert!(out.starts_with("[code:"));
}

#[test]
fn test_build_preview_never_panics_on_multibyte_utf8_every_type() {
	let _g = cap_guard();
	let content = "a\u{00e9}\u{4e2d}\u{1f600}b".repeat(30);
	for ty in [
		"build",
		"diff",
		"code_rust",
		"code_sh",
		"code",
		"search",
		"html",
		"json_array",
		"json",
		"json_list",
		"terminal",
		"text",
	] {
		let _ = build_preview(ty, &content);
	}
}

#[test]
fn test_build_preview_handles_literal_marker_shaped_content() {
	let _g = cap_guard();
	// Content that already contains marker-shaped text (e.g. a pasted
	// example transcript) must not confuse the line/byte counting or
	// panic in any branch - build_preview only ever summarizes, it never
	// re-parses content as a marker.
	let content = "before <<<CCR:fake000|text|1>>> after\nerror: boom";
	for ty in [
		"build",
		"diff",
		"code_rust",
		"code_sh",
		"code",
		"search",
		"html",
		"json_array",
		"json",
		"json_list",
		"terminal",
		"text",
	] {
		let out = build_preview(ty, content);
		assert!(!out.is_empty());
	}
}

#[test]
fn test_detect_type_never_panics_on_multibyte_utf8() {
	let _g = cap_guard();
	let content = "\u{1f600}".repeat(500);
	let _ = detect_type(&content);
}

// ── Enriched preview arms (report 09 §5): realistic fixtures, exact
// enriched output. Each preview must be self-describing `[type:...]` and
// pack decision-relevant facts. ──

const GIT_STATUS:&str = " M crates/aphrodite/src/preview.rs\n M crates/aphrodite/src/hooks.rs\nA  src/new_a.rs\nA  \
                         src/new_b.rs\nD  src/old.rs\n?? tmp/scratch\n?? tmp/other\n?? build/log";

#[test]
fn test_detect_and_preview_git_status() {
	let _g = cap_guard();
	assert_eq!(detect_semantic_type(GIT_STATUS), Some("git"));
	let p = build_preview("text", GIT_STATUS);
	// 2 M, 2 A, 1 D, 3 ?? + first 3 paths + "+5 more".
	assert_eq!(
		p,
		"[git:2M 2A 1D 3?? | crates/aphrodite/src/preview.rs crates/aphrodite/src/hooks.rs src/new_a.rs +5 more]"
	);
}

#[test]
fn test_preview_git_status_rename() {
	let _g = cap_guard();
	let c = "R  old/path.rs -> new/path.rs\nR  a.txt -> b.txt";
	let p = build_preview("git", c);
	assert!(p.starts_with("[git:2R | new/path.rs b.txt"), "got {p}");
}

const CARGO_TEST:&str = "running 221 tests\ntest foo::bar ... ok\ntest result: ok. 220 passed; 0 failed; 1 ignored; 0 \
                         measured; 0 filtered out; finished in 0.31s";

#[test]
fn test_detect_and_preview_cargo_test() {
	let _g = cap_guard();
	assert_eq!(detect_semantic_type(CARGO_TEST), Some("test"));
	let p = build_preview("text", CARGO_TEST);
	assert_eq!(p, "[test:220 pass 0 fail 1 ignored | 0.31s]");
}

#[test]
fn test_preview_test_names_first_failure() {
	let _g = cap_guard();
	let c = "running 3 tests\ntest alpha ... ok\ntest beta ... FAILED\ntest result: FAILED. 2 passed; 1 failed; 0 \
	         ignored; finished in 0.05s";
	let p = build_preview("text", c);
	assert!(p.starts_with("[test:2 pass 1 fail 0 ignored | FAIL beta"), "got {p}");
}

const LS_LONG:&str = "total 48\ndrwxr-xr-x  5 nikola staff  160 Jul 14 10:00 src\ndrwxr-xr-x  2 nikola staff   64 Jul \
                      14 10:00 tests\n-rw-r--r--  1 nikola staff 1913 Jul 14 10:00 preview.rs\n-rw-r--r--  1 nikola \
                      staff  820 Jul 14 10:00 hooks.rs\n-rw-r--r--  1 nikola staff  512 Jul 14 10:00 README.md";

#[test]
fn test_detect_and_preview_ls_long() {
	let _g = cap_guard();
	assert_eq!(detect_semantic_type(LS_LONG), Some("ls"));
	let p = build_preview("text", LS_LONG);
	// 3 files, 2 dirs; extensions .rs×2 .md×1 (the `total 48` line is skipped).
	assert_eq!(p, "[ls:3 files 2 dirs | .rs×2 .md×1]");
}

const RIPGREP:&str = "src/preview.rs:12:    let lines = content.lines().count();\nsrc/preview.rs:88:    \
                      format!(\"[terminal...\nsrc/hooks.rs:91:    let preview = \
                      crate::build_preview();\nsrc/marker.rs:49:    let mut safe = preview.replace();";

#[test]
fn test_detect_and_preview_ripgrep() {
	let _g = cap_guard();
	assert_eq!(detect_semantic_type(RIPGREP), Some("grep"));
	let p = build_preview("text", RIPGREP);
	assert_eq!(p, "[grep:4 hits in 3 files | src/preview.rs:12 …]");
}

const GIT_LOG:&str = "commit abc1234def5678\nAuthor: Nikola <n@x.io>\nDate:   Mon Jul 14\n\n    fix(preview): stop \
                      doubling\n\ncommit def5678abc1234\nAuthor: Nikola <n@x.io>\nDate:   Sun Jul 13\n\n    feat: add \
                      detector";

#[test]
fn test_detect_and_preview_git_log() {
	let _g = cap_guard();
	assert_eq!(detect_semantic_type(GIT_LOG), Some("gitlog"));
	let p = build_preview("text", GIT_LOG);
	assert_eq!(
		p,
		"[gitlog:2 commits | abc1234 fix(preview): stop doubling → def5678 feat: add detector]"
	);
}

#[test]
fn test_preview_build_surfaces_first_error() {
	let _g = cap_guard();
	let c = "   Compiling aphrodite v1.3.3\nerror[E0432]: unresolved import `crate::foo`\n  --> \
	         src/x.rs:1:5\nwarning: unused variable `y`";
	let p = build_preview("build", c);
	assert!(p.starts_with("[build:"), "got {p}");
	assert!(p.contains("E0432]: unresolved import"), "must surface first error text: {p}");
}

#[test]
fn test_preview_diff_names_files() {
	let _g = cap_guard();
	let c = "diff --git a/src/main.rs b/src/main.rs\n@@ -1,2 +1,3 @@\n+new\ndiff --git a/Cargo.toml b/Cargo.toml\n@@ \
	         -1 +1 @@\n-x\n+y";
	let p = build_preview("diff", c);
	assert!(p.contains("src/main.rs"), "got {p}");
	assert!(p.contains("Cargo.toml"), "got {p}");
}

#[test]
fn test_fallback_shows_first_line_hint() {
	let _g = cap_guard();
	let c = "some unrecognizable prose here\nline two\nline three";
	let p = build_preview("text", c);
	// No semantic shape detected -> generic fallback WITH a first-line hint.
	assert_eq!(p, "[text:3L 50B | some unrecognizable prose here]");
}

#[test]
fn test_semantic_detector_leaves_prose_alone() {
	let _g = cap_guard();
	let prose = "The quick brown fox jumps over the lazy dog.\nAnother sentence of ordinary prose.";
	assert_eq!(detect_semantic_type(prose), None);
}

// ── JSON preview (intelligent parsing, not crude `{"` count) ──

#[test]
fn test_json_preview_array_shows_keys() {
	let _g = cap_guard();
	let c = "[{\"status\":\"ok\",\"count\":42},{\"status\":\"err\",\"count\":0}]";
	let p = build_preview("json_array", c);
	assert_eq!(p, "[json:2items 1L | keys: status, count]");
}

#[test]
fn test_json_preview_object_shows_keys() {
	let _g = cap_guard();
	let c = "{\"status\":\"in_progress\",\"conclusion\":null,\"jobs\":[{\"name\":\"Test\"}]}";
	let p = build_preview("json", c);
	assert!(p.starts_with("[json:3keys"));
	assert!(p.contains("status"));
	assert!(p.contains("conclusion"));
	assert!(p.contains("jobs"));
}

#[test]
fn test_json_preview_fallback_on_unparseable() {
	let _g = cap_guard();
	let c = "not valid json {at all";
	let p = build_preview("json_array", c);
	assert!(p.starts_with("[json:~"));
}

// ── Search preview (proper regex, not bare `:` count) ──

#[test]
fn test_search_preview_counts_hits_and_files() {
	let _g = cap_guard();
	let c = "src/main.rs:12:    let x = 1;\nsrc/main.rs:42:    println!();\nsrc/lib.rs:7:    pub fn foo()";
	let p = build_preview("search", c);
	assert!(p.starts_with("[search:3 hits in 2 files | "), "got {p}");
}

#[test]
fn test_search_preview_ignores_non_search_lines() {
	let _g = cap_guard();
	let c = "src/main.rs:12:    let x = 1;\njust some prose here\nsrc/lib.rs:7:    pub fn foo()";
	let p = build_preview("search", c);
	assert!(p.contains("2 hits"), "got {p}");
}

// ── HTML preview (title, headings, links, images) ──

#[test]
fn test_html_preview_extracts_title_and_counts() {
	let _g = cap_guard();
	let c = "<!DOCTYPE html>\n<html>\n<head><title>My \
	         Page</title></head>\n<body>\n<h1>Hello</h1>\n<h2>Section</h2>\n<a href=\"/x\">link</a>\n<a \
	         href=\"/y\">link2</a>\n<img src=\"a.png\">\n</body>\n</html>";
	let p = build_preview("html", c);
	assert!(p.contains("My Page"), "must include title, got {p}");
	assert!(p.contains("2h"), "got {p}");
	assert!(p.contains("2a"), "got {p}");
	assert!(p.contains("1img"), "got {p}");
}

#[test]
fn test_new_arms_never_panic_on_pathological_input() {
	let multibyte = "a\u{00e9}\u{4e2d}\u{1f600}".repeat(20);
	let inputs = ["", "\0\0\0", multibyte.as_str()];
	for ty in ["git", "gitlog", "ls", "test", "grep"] {
		for c in inputs {
			let _ = build_preview(ty, c);
			let _ = detect_semantic_type(c);
		}
	}
}

// ── Issue #11 WS2: honest build arm (real counts, no fabricated
// success) ──

#[test]
fn test_build_preview_counts_error_warning_lines_not_substrings() {
	let _g = cap_guard();
	// One line with TWO `error` occurrences, a capitalized `Error:` line
	// (invisible to the old lowercase substring counter), an unrelated
	// `error`-shaped word, and a warning line: the OLD counter reported
	// 3E (occurrences) and missed the capitalized one entirely; the honest
	// counter reports 2 error lines + 1 warning line.
	let c = "error[E0432]: error in crate foo\nError: failed to build\nwarning: unused import\nnoerror here";
	let p = build_preview("build", c);
	assert!(p.starts_with("[build:2E 1W 4L"), "line-based counts expected, got {p}");
}

#[test]
fn test_build_preview_failing_test_never_looks_clean() {
	let _g = cap_guard();
	// ISSUE-11-PREVIEW-BATTERY #2: `test result: FAILED. 3 failed` used
	// to preview as `[build:0E 0W ...]` - a failing test run looked like
	// a clean build. The failure signal must surface.
	let c = "running 3 tests\ntest alpha ... ok\ntest result: FAILED. 1 passed; 2 failed; finished in 0.05s";
	let p = build_preview("build_output", c);
	assert!(p.starts_with("[build:0E 0W 3L | "), "failure note expected, got {p}");
	assert!(p.contains("FAILED"), "failure summary must be visible: {p}");
	assert!(!p.starts_with("[build:0E 0W 3L]"), "clean-looking summary is the bug: {p}");
}

#[test]
fn test_build_preview_passing_test_keeps_clean_summary() {
	let _g = cap_guard();
	// ISSUE-11 residual #4: a PASSING test run the classifier tagged
	// `build_output` must not render as a clean-looking `[build:0E 0W 3L]`
	// (SHALLOW - hides that the payload is a test run). It upgrades to the
	// test arm with the real tally; `0 fail` keeps the "not flagged as
	// failure" intent of the original pin.
	let c = "running 3 tests\ntest alpha ... ok\ntest result: ok. 3 passed; 0 failed; finished in 0.05s";
	let p = build_preview("build_output", c);
	assert_eq!(p, "[test:3 pass 0 fail 0 ignored | 0.05s]");
}

#[test]
fn test_build_preview_surfaces_capitalized_error_line() {
	let _g = cap_guard();
	// `Error:` (Python/Swift/clang) was invisible to the old lowercase
	// substring count; it must now count AND surface as the first error.
	let c = "   Compiling foo v0.1.0\nError: failed to run custom build command";
	let p = build_preview("build", c);
	assert!(p.starts_with("[build:1E 0W 2L | Error: failed to run"), "got {p}");
}

// ── Issue #11 WS2: honest error/linter/log arms (never success-looking,
// never a useless first line) ──

#[test]
fn test_preview_error_surfaces_the_error_line_not_traceback_header() {
	let _g = cap_guard();
	let c = "Traceback (most recent call last):\n  File \"x.py\", line 3, in <module>\nValueError: disk full";
	let p = build_preview("error", c);
	assert!(p.starts_with("[error:3L"), "got {p}");
	assert!(
		p.contains("ValueError: disk full"),
		"error arm must surface the real error: {p}"
	);
	assert!(!p.contains("Traceback"), "traceback header is not the error: {p}");
}

#[test]
fn test_preview_lint_surfaces_first_issue_line() {
	let _g = cap_guard();
	let c = "src/x.py:10:5: E501 line too long (98 > 88)\nsrc/x.py:12:1: W0611 unused import os";
	let p = build_preview("lint", c);
	assert!(p.starts_with("[lint:2L"), "got {p}");
	assert!(p.contains("E501"), "linter arm must surface the issue: {p}");
}

#[test]
fn test_preview_log_surfaces_error_signal_or_tail() {
	let _g = cap_guard();
	// Error signal wins over the tail...
	let with_err = "INFO starting\nWARN retry\nERROR connection refused\nINFO gave up";
	let p = build_preview("log", with_err);
	assert!(p.contains("connection refused"), "log arm must surface the error line: {p}");
	// ...otherwise the last non-empty line (most recent state).
	let tail = "2026-09-17T10:00:00Z INFO start\n2026-09-17T10:00:05Z INFO done";
	let p2 = build_preview("log", tail);
	assert!(p2.contains("INFO done"), "log arm must surface the tail line: {p2}");
}

// ── Issue #11 WS4: preview_max_chars cap end-to-end ──

/// Serializes tests that mutate the process-global preview cap (cargo
/// runs this module's tests concurrently; the cap is process-wide).
fn cap_guard() -> std::sync::MutexGuard<'static, ()> { crate::preview::preview_cap_test_guard() }

#[test]
fn test_preview_cap_truncates_and_keeps_bracket() {
	let _g = cap_guard();
	let prev = set_preview_max_chars(Some(30));
	let long = format!("some {} prose", "x".repeat(200));
	let p = build_preview("text", &long);
	assert!(p.chars().count() <= 30, "preview must respect the cap: {p}");
	assert!(p.ends_with(']'), "self-bracketed preview must keep its closing bracket: {p}");
	assert_eq!(p.chars().count(), 30, "truncated preview should fill the budget exactly: {p}");
	set_preview_max_chars(prev);
}

#[test]
fn test_preview_cap_unset_is_unlimited() {
	let _g = cap_guard();
	set_preview_max_chars(None);
	let long = format!("some {} prose", "y".repeat(200));
	let p = build_preview("text", &long);
	assert!(p.chars().count() > 30, "no cap -> full preview expected: {p}");
	assert!(p.contains("yyy"), "got {p}");
}

#[test]
fn test_preview_cap_multibyte_truncates_on_char_boundary() {
	let _g = cap_guard();
	let prev = set_preview_max_chars(Some(25));
	let content = format!("{}{}", "a\u{00e9}\u{4e2d}\u{1f600}".repeat(30), " end");
	let p = build_preview("text", &content);
	assert!(
		p.chars().count() <= 25,
		"multibyte truncation must stay on a char boundary: {p}"
	);
	set_preview_max_chars(prev);
}

#[test]
fn test_preview_cap_applies_to_build_arm_too() {
	let _g = cap_guard();
	let prev = set_preview_max_chars(Some(20));
	let c = "error[E0432]: unresolved import `crate::foo`\n  --> src/x.rs:1:5\nwarning: unused variable `y`";
	let p = build_preview("build", c);
	assert!(p.chars().count() <= 20, "cap must apply to every arm: {p}");
	assert!(p.ends_with(']'));
	set_preview_max_chars(prev);
}

// ── ISSUE-11 residuals #1/#2/#4 (the honest tail) ──────────

// #1: pretty-printed JSON with a `tool_result` hint previewed as a lone
// `{` (MISLEADING, the top battery offender). The generic arm must skip
// the structural brace line and show the first MEANINGFUL line.
#[test]
fn test_preview_pretty_json_lone_brace_never_previews_as_brace() {
	let _g = cap_guard();
	// Wrapper-envelope keys (name/version) keep the raw-JSON preview; the
	// generic arm must NOT show the lone `{` opener.
	let c = "{\n  \"name\": \"webapp\",\n  \"version\": \"1.0.0\",\n  \"port\": 8080\n}";
	let p = build_preview("tool_result", c);
	assert!(p.starts_with("[tool_result:5L"), "got {p}");
	assert!(
		p.contains("\"name\": \"webapp\""),
		"first meaningful line (first key) must be shown, got {p}"
	);
	assert!(!p.contains("| {]"), "lone brace preview is the bug: {p}");
}

// #1: pretty-printed JSON WITHOUT wrapper-envelope keys routes to the JSON
// arm (detection upgrade RC-C) - keys + counts, never a brace.
#[test]
fn test_preview_pretty_json_routes_to_json_arm() {
	let _g = cap_guard();
	let c = "{\n  \"web\": [\n    { \"title\": \"x\" }\n  ]\n}";
	let p = build_preview("tool_result", c);
	assert!(p.starts_with("[json:"), "pretty JSON must route to the json arm, got {p}");
	assert!(p.contains("web"), "top-level keys must be listed: {p}");
}

// #1/#4: a single-line JSON object with no hint (nested_obj_nohint /
// flat_json_nohint battery rows) used to preview as `[text:...]` SHALLOW;
// detection now routes it to the json arm.
#[test]
fn test_preview_nested_json_object_routes_to_json_arm() {
	let _g = cap_guard();
	let c = "{\"data\": {\"user\": {\"name\": \"Alice\", \"email\": \"a@b.c\"}}}";
	assert_eq!(detect_semantic_type(c), Some("json"));
	let p = build_preview("text", c);
	assert!(p.starts_with("[json:1keys 1L | data]"), "got {p}");
}

// #2: terminal arm shows the FIRST meaningful line, never the last (the
// `[terminal:7L }]` bug class). An exit-code line still wins.
#[test]
fn test_terminal_arm_shows_first_meaningful_line_not_last() {
	let _g = cap_guard();
	let c = "$ run script\noutput line\n}";
	let p = build_preview("terminal", c);
	assert!(p.starts_with("[terminal:3L $ run script]"), "got {p}");
	assert!(!p.contains("| }]"), "closing-brace preview is the bug: {p}");
}

// #2: rust code with a `terminal` hint (rust_code_hint_terminal battery
// row) - detection upgrades it to the code arm instead of `[terminal:7L }]`.
// (The code arm's fn count is the structure signal; the signature line is
// only appended when the extractor sees a return type.)
#[test]
fn test_terminal_hint_with_code_routes_to_code_arm() {
	let _g = cap_guard();
	let c = "use std::collections::HashMap;\n\nfn main() {\n    let mut map = HashMap::new();\n    map.insert(\"a\", \
	         1);\n    println!(\"{:?}\", map);\n}";
	assert_eq!(detect_semantic_type(c), Some("code"));
	let p = build_preview("terminal", c);
	assert!(
		p.starts_with("[code:1fns"),
		"code with terminal hint must get the code arm, got {p}"
	);
	assert!(
		!p.starts_with("[terminal:"),
		"terminal hint must not keep the terminal arm: {p}"
	);
}

// #4 (RC-C): raw diff with a `tool_result` hint (diff_raw battery row)
// upgrades to the diff arm with the changed file named.
#[test]
fn test_detect_diff_raw_upgrades_to_diff_arm() {
	let _g = cap_guard();
	let c = "--- a/foo.rs\n+++ b/foo.rs\n@@ -1,2 +1,3 @@\n-old line\n+new line\ncontext";
	assert_eq!(detect_semantic_type(c), Some("diff"));
	let p = build_preview("tool_result", c);
	assert!(p.starts_with("[diff:"), "raw diff must get the diff arm, got {p}");
	assert!(p.contains("foo.rs"), "changed file must be named: {p}");
}

// #4 (RC-C): yaml with a `tool_result` hint upgrades to the yaml arm.
#[test]
fn test_detect_yaml_upgrades_to_yaml_arm() {
	let _g = cap_guard();
	let c = "name: webapp\nversion: 1.0.0\nport: 8080\ndebug: true";
	assert_eq!(detect_semantic_type(c), Some("yaml"));
	let p = build_preview("tool_result", c);
	assert!(p.starts_with("[yaml:4 keys 4L | name, version, port, debug]"), "got {p}");
}

// #4 (RC-C): markdown table upgrades to the table arm with cols+rows+header.
#[test]
fn test_detect_markdown_table_upgrades_to_table_arm() {
	let _g = cap_guard();
	let c = "| Name | Age | City |\n|------|-----|------|\n| Alice | 30 | NYC |\n| Bob | 25 | LA |";
	assert_eq!(detect_semantic_type(c), Some("table"));
	let p = build_preview("tool_result", c);
	assert!(p.starts_with("[table:3 cols 3 rows | Name, Age, City]"), "got {p}");
}

// #4 (RC-C): csv, xml, markdown doc, build log all upgrade off the generic
// first-line arm.
#[test]
fn test_detect_csv_upgrades_to_csv_arm() {
	let _g = cap_guard();
	let c = "name,age,city\nalice,30,nyc\nbob,25,la\ncarol,28,sf";
	assert_eq!(detect_semantic_type(c), Some("csv"));
	let p = build_preview("tool_result", c);
	assert!(p.starts_with("[csv:4 rows 3 cols | name, age, city]"), "got {p}");
}

#[test]
fn test_detect_xml_upgrades_to_xml_arm() {
	let _g = cap_guard();
	let c = "<root>\n  <item>one</item>\n  <item>two</item>\n</root>";
	assert_eq!(detect_semantic_type(c), Some("xml"));
	let p = build_preview("tool_result", c);
	assert!(p.starts_with("[xml:3 elements 4L | <root>]"), "got {p}");
}

#[test]
fn test_detect_markdown_doc_upgrades_to_markdown_arm() {
	let _g = cap_guard();
	let c = "# Release Notes\n\n## Features\n- new previews\n- honest counts\n\n## Fixes";
	assert_eq!(detect_semantic_type(c), Some("markdown"));
	let p = build_preview("tool_result", c);
	assert!(p.starts_with("[md:"), "got {p}");
	assert!(p.contains("h1×1 h2×2"), "heading tally expected: {p}");
	assert!(p.contains("# Release Notes"), "first heading expected: {p}");
}

#[test]
fn test_detect_build_log_upgrades_to_build_arm() {
	let _g = cap_guard();
	let c = "Compiling foo v0.1.0\nCompiling bar v0.1.1\nFinished dev [unoptimized] target(s) in 0.42s";
	assert_eq!(detect_semantic_type(c), Some("build"));
	let p = build_preview("tool_result", c);
	assert_eq!(p, "[build:0E 0W 3L]");
}

// #4: git status / ls with a `tool_result` hint (git_status_raw / ls_raw
// battery rows) - the semantic detectors already knew these shapes; the
// hint made them unreachable. The upgrade set now includes `tool_result`.
#[test]
fn test_tool_result_hint_still_upgrades_git_and_ls() {
	let _g = cap_guard();
	let git = "M src/a.rs\n?? tmp/scratch\nA  src/new.rs";
	let p = build_preview("tool_result", git);
	assert!(p.starts_with("[git:"), "git status with tool_result hint: {p}");
	let p2 = build_preview("tool_result", LS_LONG);
	assert!(p2.starts_with("[ls:"), "ls listing with tool_result hint: {p2}");
}

// #4 (RC-D): a very long single line (long_single_line / repeated battery
// rows) samples head+tail instead of a bare head.
#[test]
fn test_generic_arm_long_line_samples_head_and_tail() {
	let _g = cap_guard();
	let c = "word ".repeat(2000);
	let p = build_preview("tool_result", &c);
	assert!(p.starts_with("[tool_result:1L 10000B | "), "got {p}");
	assert!(p.contains("…"), "long line must be sampled head+tail: {p}");
	assert!(p.contains("word"), "both ends are words: {p}");
	assert!(p.chars().count() < 120, "sample must stay compact: {p}");
}

// #4: a bare test log without a summary line (term_plain battery row)
// upgrades to the test arm with an honest pass/fail tally.
#[test]
fn test_bare_test_log_without_summary_gets_test_arm() {
	let _g = cap_guard();
	let c = "running 3 tests\ntest alpha ... ok\ntest beta ... ok";
	assert_eq!(detect_semantic_type(c), Some("test"));
	let p = build_preview("tool_result", c);
	assert!(p.starts_with("[test:2 pass 0 fail 0 ignored]"), "got {p}");
}

// #4: wrapper-envelope JSON (success/output/diff keys) must NOT be hijacked
// into the json arm - the raw wrapper preview is the payload (WS1).
#[test]
fn test_envelope_json_objects_are_not_hijacked_to_json_arm() {
	let _g = cap_guard();
	for c in [
		"{\"success\": true}",
		"{\"output\": \"running 10 tests\\n\\nte\", \"exit_code\": 0}",
		"{\"error\": \"file not found: foo.rs\"}",
	] {
		assert_ne!(detect_semantic_type(c), Some("json"), "envelope must be excluded: {c}");
	}
	let p = build_preview("tool_result", "{\"success\": true}");
	assert!(p.starts_with("[tool_result:"), "envelope keeps its hinted arm: {p}");
}

// #4: shell-script comments must not be mis-tagged as markdown.
#[test]
fn test_shell_comments_are_not_misdetected_as_markdown() {
	let _g = cap_guard();
	let c = "#!/bin/sh\n# build the thing\n# then run it\necho done";
	assert_eq!(detect_semantic_type(c), Some("code"), "shebang + code wins: {c}");
	let c2 = "# just a comment\n# another comment\n# third comment";
	assert_ne!(detect_semantic_type(c2), Some("markdown"), "comment-only lines are not a doc");
}
