//! Shape detection and upgrade routing.
//!
//! Detection decides which arm renders, so a detection miss shows the model the
//! generic first-line arm for content that has a much better one. These tests pin
//! both directions: a shape must be recognised, and content that merely resembles
//! a shape must not be claimed by it.

use super::*;

#[test]
fn TestSemanticDetectorLeavesProseAlone() {
	let _G = CapGuard();
	let Prose = "The quick brown fox jumps over the lazy dog.\nAnother sentence of ordinary prose.";
	assert_eq!(crate::Preview::Detect::SemanticType::Fn(Prose), None);
}

// #1: pretty-printed JSON WITHOUT wrapper-envelope keys routes to the JSON
// arm (detection upgrade RC-C) - keys + counts, never a brace.
#[test]
fn TestPreviewPrettyJSONRoutesToJSONArm() {
	let _G = CapGuard();
	let c = "{\n  \"web\": [\n    { \"title\": \"x\" }\n  ]\n}";
	let p = crate::Preview::Builder::Entry::Fn("tool_result", c);
	assert!(p.starts_with("[json:"), "pretty JSON must route to the json arm, got {p}");
	assert!(p.contains("web"), "top-level keys must be listed: {p}");
}

// #1/#4: a single-line JSON object with no hint (nested_obj_nohint /
// flat_json_nohint battery rows) used to preview as `[text:...]` SHALLOW;
// detection now routes it to the json arm.
#[test]
fn TestPreviewNestedJSONObjectRoutesToJSONArm() {
	let _G = CapGuard();
	let c = "{\"data\": {\"user\": {\"name\": \"Alice\", \"email\": \"a@b.c\"}}}";
	assert_eq!(crate::Preview::Detect::SemanticType::Fn(c), Some("json"));
	let p = crate::Preview::Builder::Entry::Fn("text", c);
	assert!(p.starts_with("[json:1keys 1L | data]"), "got {p}");
}

// #2: rust code with a `terminal` hint (rust_code_hint_terminal battery
// row) - detection upgrades it to the code arm instead of `[terminal:7L }]`.
// (The code arm's fn count is the structure signal; the signature line is
// only appended when the extractor sees a return type.)
#[test]
fn TestTerminalHintWithCodeRoutesToCodeArm() {
	let _G = CapGuard();
	let c = "use std::collections::HashMap;\n\nfn main() {\n    let mut map = HashMap::new();\n    map.insert(\"a\", \
	         1);\n    println!(\"{:?}\", map);\n}";
	assert_eq!(crate::Preview::Detect::SemanticType::Fn(c), Some("code"));
	let p = crate::Preview::Builder::Entry::Fn("terminal", c);
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
fn TestDetectDiffRawUpgradesToDiffArm() {
	let _G = CapGuard();
	let c = "--- a/foo.rs\n+++ b/foo.rs\n@@ -1,2 +1,3 @@\n-old line\n+new line\ncontext";
	assert_eq!(crate::Preview::Detect::SemanticType::Fn(c), Some("diff"));
	let p = crate::Preview::Builder::Entry::Fn("tool_result", c);
	assert!(p.starts_with("[diff:"), "raw diff must get the diff arm, got {p}");
	assert!(p.contains("foo.rs"), "changed file must be named: {p}");
}

// #4 (RC-C): yaml with a `tool_result` hint upgrades to the yaml arm.
#[test]
fn TestDetectYAMLUpgradesToYAMLArm() {
	let _G = CapGuard();
	let c = "name: webapp\nversion: 1.0.0\nport: 8080\ndebug: true";
	assert_eq!(crate::Preview::Detect::SemanticType::Fn(c), Some("yaml"));
	let p = crate::Preview::Builder::Entry::Fn("tool_result", c);
	assert!(p.starts_with("[yaml:4 keys 4L | name, version, port, debug]"), "got {p}");
}

// #4 (RC-C): markdown table upgrades to the table arm with cols+rows+header.
#[test]
fn TestDetectMarkdownTableUpgradesToTableArm() {
	let _G = CapGuard();
	let c = "| Name | Age | City |\n|------|-----|------|\n| Alice | 30 | NYC |\n| Bob | 25 | LA |";
	assert_eq!(crate::Preview::Detect::SemanticType::Fn(c), Some("table"));
	let p = crate::Preview::Builder::Entry::Fn("tool_result", c);
	assert!(p.starts_with("[table:3 cols 3 rows | Name, Age, City]"), "got {p}");
}

// #4 (RC-C): csv, xml, markdown doc, build log all upgrade off the generic
// first-line arm.
#[test]
fn TestDetectCSVUpgradesToCSVArm() {
	let _G = CapGuard();
	let c = "name,age,city\nalice,30,nyc\nbob,25,la\ncarol,28,sf";
	assert_eq!(crate::Preview::Detect::SemanticType::Fn(c), Some("csv"));
	let p = crate::Preview::Builder::Entry::Fn("tool_result", c);
	assert!(p.starts_with("[csv:4 rows 3 cols | name, age, city]"), "got {p}");
}

#[test]
fn TestDetectXMLUpgradesToXMLArm() {
	let _G = CapGuard();
	let c = "<root>\n  <item>one</item>\n  <item>two</item>\n</root>";
	assert_eq!(crate::Preview::Detect::SemanticType::Fn(c), Some("xml"));
	let p = crate::Preview::Builder::Entry::Fn("tool_result", c);
	assert!(p.starts_with("[xml:3 elements 4L | <root>]"), "got {p}");
}

#[test]
fn TestDetectMarkdownDocumentUpgradesToMarkdownArm() {
	let _G = CapGuard();
	let c = "# Release Notes\n\n## Features\n- new previews\n- honest counts\n\n## Fixes";
	assert_eq!(crate::Preview::Detect::SemanticType::Fn(c), Some("markdown"));
	let p = crate::Preview::Builder::Entry::Fn("tool_result", c);
	assert!(p.starts_with("[md:"), "got {p}");
	assert!(p.contains("h1×1 h2×2"), "heading tally expected: {p}");
	assert!(p.contains("# Release Notes"), "first heading expected: {p}");
}

#[test]
fn TestDetectBuildLogUpgradesToBuildArm() {
	let _G = CapGuard();
	let c = "Compiling foo v0.1.0\nCompiling bar v0.1.1\nFinished dev [unoptimized] target(s) in 0.42s";
	assert_eq!(crate::Preview::Detect::SemanticType::Fn(c), Some("build"));
	let p = crate::Preview::Builder::Entry::Fn("tool_result", c);
	assert_eq!(p, "[build:0E 0W 3L]");
}

// #4: git status / ls with a `tool_result` hint (git_status_raw / ls_raw
// battery rows) - the semantic detectors already knew these shapes; the
// hint made them unreachable. The upgrade set now includes `tool_result`.
#[test]
fn TestToolResultHintStillUpgradesGitAndLs() {
	let _G = CapGuard();
	let Git = "M src/a.rs\n?? tmp/scratch\nA  src/new.rs";
	let p = crate::Preview::Builder::Entry::Fn("tool_result", Git);
	assert!(p.starts_with("[git:"), "git status with tool_result hint: {p}");
	let P2 = crate::Preview::Builder::Entry::Fn("tool_result", LsLong);
	assert!(P2.starts_with("[ls:"), "ls listing with tool_result hint: {P2}");
}

// #4: a bare test log without a summary line (term_plain battery row)
// upgrades to the test arm with an honest pass/fail tally.
#[test]
fn TestBareTestLogWithoutSummaryGetsTestArm() {
	let _G = CapGuard();
	let c = "running 3 tests\ntest alpha ... ok\ntest beta ... ok";
	assert_eq!(crate::Preview::Detect::SemanticType::Fn(c), Some("test"));
	let p = crate::Preview::Builder::Entry::Fn("tool_result", c);
	assert!(p.starts_with("[test:2 pass 0 fail 0 ignored]"), "got {p}");
}

// #4: wrapper-envelope JSON (success/output/diff keys) must NOT be hijacked
// into the json arm - the raw wrapper preview is the payload (WS1).
#[test]
fn TestEnvelopeJSONObjectsAreNotHijackedToJSONArm() {
	let _G = CapGuard();
	for c in [
		"{\"success\": true}",
		"{\"output\": \"running 10 tests\\n\\nte\", \"exit_code\": 0}",
		"{\"error\": \"file not found: foo.rs\"}",
	] {
		assert_ne!(
			crate::Preview::Detect::SemanticType::Fn(c),
			Some("json"),
			"envelope must be excluded: {c}"
		);
	}
	let p = crate::Preview::Builder::Entry::Fn("tool_result", "{\"success\": true}");
	assert!(p.starts_with("[tool_result:"), "envelope keeps its hinted arm: {p}");
}

// #4: shell-script comments must not be mis-tagged as markdown.
#[test]
fn TestShellCommentsAreNotMisdetectedAsMarkdown() {
	let _G = CapGuard();
	let c = "#!/bin/sh\n# build the thing\n# then run it\necho done";
	assert_eq!(
		crate::Preview::Detect::SemanticType::Fn(c),
		Some("code"),
		"shebang + code wins: {c}"
	);
	let C2 = "# just a comment\n# another comment\n# third comment";
	assert_ne!(
		crate::Preview::Detect::SemanticType::Fn(C2),
		Some("markdown"),
		"comment-only lines are not a doc"
	);
}
