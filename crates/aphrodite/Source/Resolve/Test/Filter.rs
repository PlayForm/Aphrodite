//! Query narrowing.
//!
//! Pins the three outcomes a retrieve query can produce: no query at all (content
//! untouched), matches (only the matching lines), and no match (the full content
//! behind a marker prefix, never an empty string).

use crate::Resolve::Filter::Fn as FilterLines;

#[test]
fn TestFilterEmptyQuery() {
	assert_eq!(FilterLines("hello\nworld", ""), "hello\nworld");
}

#[test]
fn TestFilterMatching() {
	let Content = "line one\nerror: broke\nline two\nERROR: fatal\n";
	let Result = FilterLines(Content, "error");
	assert!(Result.contains("error: broke"));
	assert!(Result.contains("ERROR: fatal"));
	assert!(!Result.contains("line one"));
}

#[test]
fn TestFilterNoMatch() {
	let Content = "hello\nworld\n";
	let Result = FilterLines(Content, "nonexistent");
	assert!(Result.contains("[aphrodite: no lines matched"));
}
