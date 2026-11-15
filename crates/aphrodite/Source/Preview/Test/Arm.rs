//! Exact per-arm output.
//!
//! Each test here pins one arm's rendered string against a realistic fixture.
//! These are the assertions that make the preview a contract: the model reads
//! these bytes, so a reordered field or a dropped path is a behavioural change
//! even when the stored payload is untouched.

use super::*;

#[test]
fn TestDetectAndPreviewGitStatus() {
	let _G = CapGuard();
	assert_eq!(DetectSemanticType(GitStatus), Some("git"));
	let p = BuildPreview("text", GitStatus);
	// 2 M, 2 A, 1 D, 3 ?? + first 3 paths + "+5 more".
	assert_eq!(
		p,
		"[git:2M 2A 1D 3?? | crates/aphrodite/src/preview.rs crates/aphrodite/src/hooks.rs src/new_a.rs +5 more]"
	);
}

#[test]
fn TestPreviewGitStatusRename() {
	let _G = CapGuard();
	let c = "R  old/path.rs -> new/path.rs\nR  a.txt -> b.txt";
	let p = BuildPreview("git", c);
	assert!(p.starts_with("[git:2R | new/path.rs b.txt"), "got {p}");
}

#[test]
fn TestDetectAndPreviewCargoTest() {
	let _G = CapGuard();
	assert_eq!(DetectSemanticType(CargoTest), Some("test"));
	let p = BuildPreview("text", CargoTest);
	assert_eq!(p, "[test:220 pass 0 fail 1 ignored | 0.31s]");
}

#[test]
fn TestPreviewTestNamesFirstFailure() {
	let _G = CapGuard();
	let c = "running 3 tests\ntest alpha ... ok\ntest beta ... FAILED\ntest result: FAILED. 2 passed; 1 failed; 0 \
	         ignored; finished in 0.05s";
	let p = BuildPreview("text", c);
	assert!(p.starts_with("[test:2 pass 1 fail 0 ignored | FAIL beta"), "got {p}");
}

#[test]
fn TestDetectAndPreviewLsLong() {
	let _G = CapGuard();
	assert_eq!(DetectSemanticType(LsLong), Some("ls"));
	let p = BuildPreview("text", LsLong);
	// 3 files, 2 dirs; extensions .rs×2 .md×1 (the `total 48` line is skipped).
	assert_eq!(p, "[ls:3 files 2 dirs | .rs×2 .md×1]");
}

#[test]
fn TestDetectAndPreviewRipgrep() {
	let _G = CapGuard();
	assert_eq!(DetectSemanticType(Ripgrep), Some("grep"));
	let p = BuildPreview("text", Ripgrep);
	assert_eq!(p, "[grep:4 hits in 3 files | src/preview.rs:12 ...]");
}

#[test]
fn TestDetectAndPreviewGitLog() {
	let _G = CapGuard();
	assert_eq!(DetectSemanticType(GitLog), Some("gitlog"));
	let p = BuildPreview("text", GitLog);
	assert_eq!(
		p,
		"[gitlog:2 commits | abc1234 fix(preview): stop doubling → def5678 feat: add detector]"
	);
}

#[test]
fn TestPreviewBuildSurfacesFirstError() {
	let _G = CapGuard();
	let c = "   Compiling aphrodite v1.3.3\nerror[E0432]: unresolved import `crate::foo`\n  --> \
	         src/x.rs:1:5\nwarning: unused variable `y`";
	let p = BuildPreview("build", c);
	assert!(p.starts_with("[build:"), "got {p}");
	assert!(p.contains("E0432]: unresolved import"), "must surface first error text: {p}");
}

#[test]
fn TestPreviewDiffNamesFiles() {
	let _G = CapGuard();
	let c = "diff --git a/src/main.rs b/src/main.rs\n@@ -1,2 +1,3 @@\n+new\ndiff --git a/Cargo.toml b/Cargo.toml\n@@ \
	         -1 +1 @@\n-x\n+y";
	let p = BuildPreview("diff", c);
	assert!(p.contains("src/main.rs"), "got {p}");
	assert!(p.contains("Cargo.toml"), "got {p}");
}

#[test]
fn TestFallbackShowsFirstLineHint() {
	let _G = CapGuard();
	let c = "some unrecognizable prose here\nline two\nline three";
	let p = BuildPreview("text", c);
	// No semantic shape detected -> generic fallback WITH a first-line hint.
	assert_eq!(p, "[text:3L 50B | some unrecognizable prose here]");
}

#[test]
fn TestJSONPreviewArrayShowsKeys() {
	let _G = CapGuard();
	let c = "[{\"status\":\"ok\",\"count\":42},{\"status\":\"err\",\"count\":0}]";
	let p = BuildPreview("json_array", c);
	assert_eq!(p, "[json:2items 1L | keys: status, count]");
}

#[test]
fn TestJSONPreviewObjectShowsKeys() {
	let _G = CapGuard();
	let c = "{\"status\":\"in_progress\",\"conclusion\":null,\"jobs\":[{\"name\":\"Test\"}]}";
	let p = BuildPreview("json", c);
	assert!(p.starts_with("[json:3keys"));
	assert!(p.contains("status"));
	assert!(p.contains("conclusion"));
	assert!(p.contains("jobs"));
}

#[test]
fn TestJSONPreviewFallbackOnUnparseable() {
	let _G = CapGuard();
	let c = "not valid json {at all";
	let p = BuildPreview("json_array", c);
	assert!(p.starts_with("[json:~"));
}

#[test]
fn TestSearchPreviewCountsHitsAndFiles() {
	let _G = CapGuard();
	let c = "src/main.rs:12:    let x = 1;\nsrc/main.rs:42:    println!();\nsrc/lib.rs:7:    pub fn foo()";
	let p = BuildPreview("search", c);
	assert!(p.starts_with("[search:3 hits in 2 files | "), "got {p}");
}

#[test]
fn TestSearchPreviewIgnoresNonSearchLines() {
	let _G = CapGuard();
	let c = "src/main.rs:12:    let x = 1;\njust some prose here\nsrc/lib.rs:7:    pub fn foo()";
	let p = BuildPreview("search", c);
	assert!(p.contains("2 hits"), "got {p}");
}

#[test]
fn TestHTMLPreviewExtractsTitleAndCounts() {
	let _G = CapGuard();
	let c = "<!DOCTYPE html>\n<html>\n<head><title>My \
	         Page</title></head>\n<body>\n<h1>Hello</h1>\n<h2>Section</h2>\n<a href=\"/x\">link</a>\n<a \
	         href=\"/y\">link2</a>\n<img src=\"a.png\">\n</body>\n</html>";
	let p = BuildPreview("html", c);
	assert!(p.contains("My Page"), "must include title, got {p}");
	assert!(p.contains("2h"), "got {p}");
	assert!(p.contains("2a"), "got {p}");
	assert!(p.contains("1img"), "got {p}");
}

#[test]
fn TestTerminalArmShowsFirstMeaningfulLineNotLast() {
	let _G = CapGuard();
	let c = "$ run script\noutput line\n}";
	let p = BuildPreview("terminal", c);
	assert!(p.starts_with("[terminal:3L $ run script]"), "got {p}");
	assert!(!p.contains("| }]"), "closing-brace preview is the bug: {p}");
}

#[test]
fn TestGenericArmLongLineSamplesHeadAndTail() {
	let _G = CapGuard();
	let c = "word ".repeat(2000);
	let p = BuildPreview("tool_result", &c);
	assert!(p.starts_with("[tool_result:1L 10000B | "), "got {p}");
	assert!(p.contains("..."), "long line must be sampled head+tail: {p}");
	assert!(p.contains("word"), "both ends are words: {p}");
	assert!(p.chars().count() < 120, "sample must stay compact: {p}");
}
