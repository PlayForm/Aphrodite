//! The chain-split battery: separator handling, the marker round trip, and the
//! per-segment error hints.

use crate::ChainSplit::{
	ErrorHint::Fn::Fn as Hint,
	Marked::{Command::Fn::Fn as Command, Output::Fn::Fn as Output},
	Segment::Const::Marker,
	Split::Fn::Fn as SplitChain,
};

#[test]
fn Separator() {
	let Segs = SplitChain("cd x && cargo build ; cargo test").unwrap();
	assert_eq!(Segs.len(), 3);
	assert_eq!(Segs[0].Command, "cd x");
	assert_eq!(Segs[1].Command, "cargo build");
	assert_eq!(Segs[2].Command, "cargo test");
}

#[test]
fn RespectsQuotes() {
	let Segs = SplitChain("echo 'a && b' ; echo \"c ; d\"").unwrap();
	assert_eq!(Segs.len(), 2);
	assert_eq!(Segs[0].Command, "echo 'a && b'");
	assert_eq!(Segs[1].Command, "echo \"c ; d\"");
}

#[test]
fn Single() {
	assert!(SplitChain("cargo build").is_none());
	assert!(SplitChain("").is_none());
}

#[test]
fn HeredocRefused() {
	assert!(SplitChain("cat << EOF && echo hi").is_none());
}

#[test]
fn Roundtrip() {
	let Segs = SplitChain("a && b && c").unwrap();
	let Marked = Command(&Segs);
	assert!(Marked.contains("__APHRODITE_SEG__"));
	// Markers go to STDERR, never stdout: redirected files stay clean.
	assert!(Marked.contains("1>&2"));
	assert!(!Marked.contains("echo __APHRODITE_SEG__ ;"));
	let Out = format!("seg a output\n{}\nseg b output\n{}\nseg c output\n", Marker, Marker);
	let Parts = Output(&Out);
	assert_eq!(Parts.len(), 3);
	assert_eq!(Parts[0].Index, 0);
	assert_eq!(Parts[0].Text, "seg a output");
	assert_eq!(Parts[1].Index, 1);
	assert_eq!(Parts[1].Text, "seg b output");
	assert_eq!(Parts[2].Index, 2);
	assert_eq!(Parts[2].Text, "seg c output");
}

#[test]
fn NewlineSeparator() {
	let Segs = SplitChain("cd /tmp\ngit status\necho done").unwrap();
	assert_eq!(Segs.len(), 3);
}

// ── Tier 3: per-segment error hints ──

#[test]
fn HintClean() {
	assert_eq!(Hint("everything fine\nno problems here"), None);
	assert_eq!(Hint(""), None);
}

#[test]
fn HintSignal() {
	let Out = "compiling...\nerror: could not compile `demo`\nfailed\n";
	assert_eq!(Hint(Out).as_deref(), Some("error: could not compile `demo`"));
}

#[test]
fn Prefer() {
	let Out = "warning: unused\nfatal: something\nerror[E0432]: unresolved import `x`\n";
	assert_eq!(Hint(Out).as_deref(), Some("error[E0432]: unresolved import `x`"));
}

#[test]
fn Exit() {
	assert_eq!(Hint("done\n").as_deref(), None);
	assert_eq!(Hint("make: *** [all] Error 2\n").as_deref(), Some("make: *** [all] Error 2"));
}

#[test]
fn HintTruncate() {
	let long = format!("error: {}", "x".repeat(200));
	let Hint = Hint(&long).unwrap();
	assert!(Hint.len() <= 60);
	assert!(Hint.starts_with("error: "));
}
