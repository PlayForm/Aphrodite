//! The injected context string: what actually travels to the model.

use std::collections::HashMap;

use crate::Directive::{
	Build::Fn as BuildDirectiveContext,
	Model::{MaximumCombinedChars, MaximumDirectiveChars, Struct as Directive},
};

fn Active(name:&str, Content:&str) -> (HashMap<String, Directive>, Vec<String>) {
	let mut all = HashMap::new();
	all.insert(
		name.to_string(),
		Directive { name:name.to_string(), Content:Content.to_string() },
	);
	let Active = vec![name.to_string()];
	(all, Active)
}

#[test]
fn TestBuildEmpty() {
	let all = HashMap::new();
	let Context = BuildDirectiveContext(&all, &[]);
	assert!(Context.is_empty());
}

#[test]
fn TestBuildWithActive() {
	let mut all = HashMap::new();
	all.insert(
		"focus".into(),
		Directive { name:"focus".into(), Content:"stay concise\nuse 1-2 tools".into() },
	);
	let Context = BuildDirectiveContext(&all, &["focus".into()]);
	assert!(Context.contains("[directives: focus]"));
	assert!(Context.contains("focus:\n"));
	assert!(Context.contains("stay concise"));
	assert!(Context.contains("use 1-2 tools"));
}

// ── 01-F5: real directive files use a leading `#` on multiple lines
// (not just the title) - injection must strip it from every line and
// carry the bullet body through, not just the first line. ──
#[test]
fn TestBuildWithActiveInjectsFullBodyNotJustFirstLine() {
	let mut all = HashMap::new();
	all.insert(
		"focus".into(),
		Directive {
			name:"focus".into(),
			Content:"# focus - stay targeted, minimal tool usage\n\n# Each turn: use at most 1-2 tools.\n\n- One \
			         primary action per turn\n- Prefer aphrodite_retrieve over re-reading"
				.into(),
		},
	);
	let Context = BuildDirectiveContext(&all, &["focus".into()]);
	assert!(
		Context.contains("Each turn: use at most 1-2 tools."),
		"body line missing: {Context}"
	);
	assert!(
		Context.contains("One primary action per turn"),
		"bullet line missing: {Context}"
	);
	assert!(!Context.contains('#'), "leading # markers must be stripped: {Context}");
}

#[test]
fn TestBuildWithActiveCapsCombinedOutput() {
	let mut all = HashMap::new();
	all.insert(
		"big".into(),
		Directive { name:"big".into(), Content:"x".repeat(MaximumDirectiveChars) },
	);
	all.insert(
		"also-big".into(),
		Directive { name:"also-big".into(), Content:"y".repeat(MaximumDirectiveChars) },
	);
	let Context = BuildDirectiveContext(&all, &["big".into(), "also-big".into()]);
	assert!(
		Context.len() <= MaximumCombinedChars + 10,
		"combined output must respect the cap: {} chars",
		Context.len()
	);
}

// ── T19: the combined-output gate must be CHAR-based, not byte-based.
// Em-dashes are 3 bytes each, so content that fits the 4000-char cap
// still exceeds it in bytes - the old `out.len() > MAX_COMBINED_CHARS`
// check truncated it anyway. The result must stay within the char cap.
#[test]
fn TestCombinedCharCapCountsCharsNotBytes() {
	let (all, Active) = Active("focus", &"\u{2014}".repeat(3000));
	let Out = BuildDirectiveContext(&all, &Active);
	// Char count (prefix + 3000 dashes + newline) is within the cap...
	assert!(
		Out.chars().count() <= MaximumCombinedChars,
		"char-count output must stay within the char cap"
	);
	// ...but the byte length exceeds it, which would have tripped the
	// old byte-based gate and truncated valid content.
	assert!(Out.len() > MaximumCombinedChars, "byte length exceeds the cap");
	// Untruncated: full content present, no ellipsis.
	assert!(!Out.contains("..."), "within-cap content must not be truncated");
	assert!(Out.ends_with("\u{2014}\n"));
}

// ── T19: genuinely over-cap content is still truncated, char-exact:
// exactly MAX_COMBINED_CHARS chars of content survive before the
// ellipsis newline.
#[test]
fn TestCombinedCharCapTruncatesOverCapContent() {
	let (all, Active) = Active("focus", &"-".repeat(4100));
	let Out = BuildDirectiveContext(&all, &Active);
	assert!(Out.ends_with("...\n"), "over-cap content must be truncated");
	let ContentPart = Out.trim_end_matches("...\n");
	assert_eq!(ContentPart.chars().count(), MaximumCombinedChars);
	assert_eq!(Out.chars().count(), MaximumCombinedChars + 4);
}
