//! The query filter: pass-through, case-insensitive matching, the no-match notice and the
//! 512-char query cap.
//!
//! The filter answers a `Cow`, so the pass-through case is asserted through `as_ref()`:
//! comparing the `Cow` itself would leave the comparison to a blanket `PartialEq` and hide
//! which arm produced the value.

use crate::Retrieve::Filter::Fn::Fn as FilterContent;

// ── T9: filter_content ────────────────────────────────────────
#[test]
fn TestFilterContentNoQueryReturnsUnchanged() {
	assert_eq!(FilterContent("a\nb\nc", None).as_ref(), "a\nb\nc");
	assert_eq!(FilterContent("a\nb\nc", Some("")).as_ref(), "a\nb\nc");
}

#[test]
fn TestFilterContentMatch() {
	let Content = "alpha\nbeta error\ngamma\n";
	let Result = FilterContent(Content, Some("error"));
	assert_eq!(Result.as_ref(), "beta error");
}

#[test]
fn TestFilterContentNoMatchMessage() {
	let Content = "alpha\nbeta\n";
	let Result = FilterContent(Content, Some("nonexistent"));
	assert!(Result.contains("no lines matching"));
	assert!(Result.contains("2 lines"));
}

#[test]
fn TestFilterContentCaseInsensitive() {
	let Content = "ALPHA line\nbeta line\n";
	let Result = FilterContent(Content, Some("alpha"));
	assert_eq!(Result.as_ref(), "ALPHA line");
}

#[test]
fn TestFilterContentQueryOver512CharsIsTruncated() {
	let LongQuery = "x".repeat(600);
	let Content = "some line with lots of x's\n";
	// Must not panic on a query longer than the content; truncates to 512 chars.
	let Result = FilterContent(Content, Some(&LongQuery));
	assert!(Result.contains("no lines matching"));
}
