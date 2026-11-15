//! The never-misleading pins.
//!
//! A preview that understates a failure is worse than no preview: it tells the
//! model the work succeeded. These tests pin the honesty rules - counts are line
//! based, a failing run can never look clean, and an arm surfaces the payload
//! rather than the frame around it.

use super::*;

#[test]
fn TestBuildPreviewCountsErrorWarningLinesNotSubstrings() {
	let _G = CapGuard();
	// One line with TWO `error` occurrences, a capitalized `Error:` line
	// (invisible to the old lowercase substring counter), an unrelated
	// `error`-shaped word, and a warning line: the OLD counter reported
	// 3E (occurrences) and missed the capitalized one entirely; the honest
	// counter reports 2 error lines + 1 warning line.
	let c = "error[E0432]: error in crate foo\nError: failed to build\nwarning: unused import\nnoerror here";
	let p = BuildPreview("build", c);
	assert!(p.starts_with("[build:2E 1W 4L"), "line-based counts expected, got {p}");
}

#[test]
fn TestBuildPreviewFailingTestNeverLooksClean() {
	let _G = CapGuard();
	// ISSUE-11-PREVIEW-BATTERY #2: `test result: FAILED. 3 failed` used
	// to preview as `[build:0E 0W ...]` - a failing test run looked like
	// a clean build. The failure signal must surface.
	let c = "running 3 tests\ntest alpha ... ok\ntest result: FAILED. 1 passed; 2 failed; finished in 0.05s";
	let p = BuildPreview("build_output", c);
	assert!(p.starts_with("[build:0E 0W 3L | "), "failure note expected, got {p}");
	assert!(p.contains("FAILED"), "failure summary must be visible: {p}");
	assert!(!p.starts_with("[build:0E 0W 3L]"), "clean-looking summary is the bug: {p}");
}

#[test]
fn TestBuildPreviewPassingTestKeepsCleanSummary() {
	let _G = CapGuard();
	// ISSUE-11 residual #4: a PASSING test run the classifier tagged
	// `build_output` must not render as a clean-looking `[build:0E 0W 3L]`
	// (SHALLOW - hides that the payload is a test run). It upgrades to the
	// test arm with the real tally; `0 fail` keeps the "not flagged as
	// failure" intent of the original pin.
	let c = "running 3 tests\ntest alpha ... ok\ntest result: ok. 3 passed; 0 failed; finished in 0.05s";
	let p = BuildPreview("build_output", c);
	assert_eq!(p, "[test:3 pass 0 fail 0 ignored | 0.05s]");
}

#[test]
fn TestBuildPreviewSurfacesCapitalizedErrorLine() {
	let _G = CapGuard();
	// `Error:` (Python/Swift/clang) was invisible to the old lowercase
	// substring count; it must now count AND surface as the first error.
	let c = "   Compiling foo v0.1.0\nError: failed to run custom build command";
	let p = BuildPreview("build", c);
	assert!(p.starts_with("[build:1E 0W 2L | Error: failed to run"), "got {p}");
}

#[test]
fn TestPreviewErrorSurfacesTheErrorLineNotTracebackHeader() {
	let _G = CapGuard();
	let c = "Traceback (most recent call last):\n  File \"x.py\", line 3, in <module>\nValueError: disk full";
	let p = BuildPreview("error", c);
	assert!(p.starts_with("[error:3L"), "got {p}");
	assert!(
		p.contains("ValueError: disk full"),
		"error arm must surface the real error: {p}"
	);
	assert!(!p.contains("Traceback"), "traceback header is not the error: {p}");
}

#[test]
fn TestPreviewLintSurfacesFirstIssueLine() {
	let _G = CapGuard();
	let c = "src/x.py:10:5: E501 line too long (98 > 88)\nsrc/x.py:12:1: W0611 unused import os";
	let p = BuildPreview("lint", c);
	assert!(p.starts_with("[lint:2L"), "got {p}");
	assert!(p.contains("E501"), "linter arm must surface the issue: {p}");
}

#[test]
fn TestPreviewLogSurfacesErrorSignalOrTail() {
	let _G = CapGuard();
	// Error signal wins over the tail...
	let WithError = "INFO starting\nWARN retry\nERROR connection refused\nINFO gave up";
	let p = BuildPreview("log", WithError);
	assert!(p.contains("connection refused"), "log arm must surface the error line: {p}");
	// ...otherwise the last non-empty line (most recent state).
	let Tail = "2026-09-17T10:00:00Z INFO start\n2026-09-17T10:00:05Z INFO done";
	let P2 = BuildPreview("log", Tail);
	assert!(P2.contains("INFO done"), "log arm must surface the tail line: {P2}");
}

// #1: pretty-printed JSON with a `tool_result` hint previewed as a lone
// `{` (MISLEADING, the top battery offender). The generic arm must skip
// the structural brace line and show the first MEANINGFUL line.
#[test]
fn TestPreviewPrettyJSONLoneBraceNeverPreviewsAsBrace() {
	let _G = CapGuard();
	// Wrapper-envelope keys (name/version) keep the raw-JSON preview; the
	// generic arm must NOT show the lone `{` opener.
	let c = "{\n  \"name\": \"webapp\",\n  \"version\": \"1.0.0\",\n  \"port\": 8080\n}";
	let p = BuildPreview("tool_result", c);
	assert!(p.starts_with("[tool_result:5L"), "got {p}");
	assert!(
		p.contains("\"name\": \"webapp\""),
		"first meaningful line (first key) must be shown, got {p}"
	);
	assert!(!p.contains("| {]"), "lone brace preview is the bug: {p}");
}
