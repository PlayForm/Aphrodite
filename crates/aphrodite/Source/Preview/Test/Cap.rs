//! `PreviewMaximumChars` cap, end to end.
//!
//! The cap is the one guarantee every arm shares, so it is tested against a
//! generic arm, a structured arm and multibyte content: a cap that only works on
//! ASCII, or only on the generic path, is not a cap.

use super::*;

#[test]
fn TestPreviewCapTruncatesAndKeepsBracket() {
	let _G = CapGuard();
	let Previous = crate::Preview::State::Set::Fn(Some(30));
	let long = format!("some {} prose", "x".repeat(200));
	let p = crate::Preview::Builder::Entry::Fn("text", &long);
	assert!(p.chars().count() <= 30, "preview must respect the cap: {p}");
	assert!(p.ends_with(']'), "self-bracketed preview must keep its closing bracket: {p}");
	assert_eq!(p.chars().count(), 30, "truncated preview should fill the budget exactly: {p}");
	crate::Preview::State::Set::Fn(Previous);
}

#[test]
fn TestPreviewCapUnsetIsUnlimited() {
	let _G = CapGuard();
	crate::Preview::State::Set::Fn(None);
	let long = format!("some {} prose", "y".repeat(200));
	let p = crate::Preview::Builder::Entry::Fn("text", &long);
	assert!(p.chars().count() > 30, "no cap -> full preview expected: {p}");
	assert!(p.contains("yyy"), "got {p}");
}

#[test]
fn TestPreviewCapMultibyteTruncatesOnCharBoundary() {
	let _G = CapGuard();
	let Previous = crate::Preview::State::Set::Fn(Some(25));
	let Content = format!("{}{}", "a\u{00e9}\u{4e2d}\u{1f600}".repeat(30), " end");
	let p = crate::Preview::Builder::Entry::Fn("text", &Content);
	assert!(
		p.chars().count() <= 25,
		"multibyte truncation must stay on a char boundary: {p}"
	);
	crate::Preview::State::Set::Fn(Previous);
}

#[test]
fn TestPreviewCapAppliesToBuildArmToo() {
	let _G = CapGuard();
	let Previous = crate::Preview::State::Set::Fn(Some(20));
	let c = "error[E0432]: unresolved import `crate::foo`\n  --> src/x.rs:1:5\nwarning: unused variable `y`";
	let p = crate::Preview::Builder::Entry::Fn("build", c);
	assert!(p.chars().count() <= 20, "cap must apply to every arm: {p}");
	assert!(p.ends_with(']'));
	crate::Preview::State::Set::Fn(Previous);
}
