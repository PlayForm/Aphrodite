//! The paging contract: byte-exact full-document round trips, the explicit
//! limit's 10,000-line cap, and the `[lines a-b/total]` header format.

use crate::Retrieve::Page::Fn::Fn as Paginate;

// ── T9: paginate ──────────────────────────────────────────────
#[test]
fn TestPaginateOffsetZeroFullDocumentNoHeader() {
	let Content = "a\nb\nc";
	let (Result, Truncated) = Paginate(Content, 0, 0).unwrap();
	assert_eq!(Result, "a\nb\nc");
	assert!(!Truncated);
}

// ── 02-F4: `str::lines()` discards the final line terminator and
// `join("\n")` never restores it - a full-document retrieval of content
// ending in "\n" (nearly every source file) used to come back one byte
// short, breaking the content-addressing round-trip. ──
#[test]
fn TestPaginatePreservesTrailingNewlineOnFullDocument() {
	let Content = "a\nb\nc\n";
	let (Result, Truncated) = Paginate(Content, 0, 0).unwrap();
	assert_eq!(Result, Content, "full-document retrieval must return the exact original bytes");
	assert!(!Truncated);
}

#[test]
fn TestPaginatePreservesTrailingNewlineOnPartialWindowReachingEnd() {
	let Content = "a\nb\nc\n";
	// offset=1 covers lines b,c through the end - the header is expected,
	// but the trailing newline must still survive.
	let (Result, Truncated) = Paginate(Content, 1, 0).unwrap();
	assert_eq!(Result, "[lines 2-3/3]\nb\nc\n");
	assert!(Truncated);
}

// ── report 02 T20 (F20): an empty stored document is valid, not an
// out-of-range offset. ──
#[test]
fn TestPaginateEmptyContentReturnsEmptyOkNotError() {
	assert_eq!(Paginate("", 0, 0), Ok((String::new(), false)));
}

#[test]
fn TestPaginateOffsetAtOrPastTotalIsError() {
	let Content = "a\nb\nc"; // 3 lines
	assert!(Paginate(Content, 3, 0).is_err());
	assert!(Paginate(Content, 10, 0).is_err());
	let Error = Paginate(Content, 3, 0).unwrap_err();
	assert!(Error.contains("out of range"));
	assert!(Error.contains("3 lines"));
}

// ── 02-F5: an explicit `Limit` is clamped to the 10_000-line server cap;
// `limit:0` requests the FULL document (the round-trip contract: a
// full-document retrieval must return the exact original bytes, so they
// hash back to the marker's own hash). The wide_5k_keys.json bench
// finding was a 20,002-line document being windowed to the first 10,000
// lines with a `[lines 1-10000/20002]` header, i.e. NOT the original. ──
#[test]
fn TestPaginateLimitZeroReturnsFullDocumentUnderCap() {
	let Content = (0..5).map(|i| i.to_string()).collect::<Vec<_>>().join("\n");
	let (Result, Truncated) = Paginate(&Content, 0, 0).unwrap();
	assert_eq!(Result, Content);
	assert!(!Truncated);
}

#[test]
fn TestPaginateLimitZeroReturnsFullDocumentOver10000LinesVerbatim() {
	// Regression for the wide_5k_keys.json finding: limit:0 (the bench's
	// "no pagination" request) must return the whole 20,002-line document
	// byte-identical, with no `[lines ...]` header and truncated=false.
	let Content = (0..20_002)
		.map(|i| format!("key_{i:05} = {}", i))
		.collect::<Vec<_>>()
		.join("\n");
	assert!(Content.lines().count() > 10_000);
	let (Result, Truncated) = Paginate(&Content, 0, 0).unwrap();
	assert_eq!(Result, Content, "full-document retrieval must return the exact original bytes");
	assert!(!Truncated, "limit:0 never truncates");
}

#[test]
fn TestPaginateExplicitLimitClampedTo10000Maximum() {
	let Content = (0..5).map(|i| i.to_string()).collect::<Vec<_>>().join("\n");
	// A huge explicit limit must behave the same as 10_000 (capped).
	let (Result, Truncated) = Paginate(&Content, 0, 999_999).unwrap();
	assert_eq!(Result, Content);
	assert!(!Truncated);
}

#[test]
fn TestPaginateExplicitLimitOverCapTruncatesWideDocument() {
	// An EXPLICIT limit above the cap still truncates: the safety cap
	// (02-F5) applies to explicit limits, not to limit:0.
	let Content = (0..20_002)
		.map(|i| format!("key_{i:05} = {}", i))
		.collect::<Vec<_>>()
		.join("\n");
	let (Result, Truncated) = Paginate(&Content, 0, 999_999).unwrap();
	assert!(Truncated);
	assert!(Result.starts_with("[lines 1-10000/20002]"));
	assert!(Result.len() < Content.len());
}

#[test]
fn TestPaginateWindowHeaderFormat() {
	let Content = "a\nb\nc\nd\ne"; // 5 lines
	let (Result, Truncated) = Paginate(Content, 1, 2).unwrap();
	assert_eq!(Result, "[lines 2-3/5]\nb\nc");
	assert!(Truncated);
}

// The old 02-F5 `limit: 999_999 == limit: 0` equivalence test is replaced
// above by `TestPaginateExplicitLimitClampedTo10000Maximum` +
// `TestPaginateExplicitLimitOverCapTruncatesWideDocument`: an
// explicit limit is still clamped to 10_000, but `limit:0` now returns
// the full document.
