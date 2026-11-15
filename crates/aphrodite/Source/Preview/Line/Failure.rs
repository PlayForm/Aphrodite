//! FAILED-state line predicate.
//!
//! A test run has two ways of saying it failed: a `FAILED` marker, or a count
//! line whose failed total is non-zero. Both have to be recognised, and `0
//! failed` must never be - otherwise a clean run previews as a failure, which
//! is the worst possible lie a preview can tell.

use crate::Preview::Text::NumberBefore::Casefold;

/// True for a line that signals a FAILED state.
///
/// Matches a `FAILED`/`FAIL` marker, a `--- FAIL:` test failure, or a
/// NON-ZERO `N failed` count. A clean `0 failed` summary does not match, so a
/// passing run stays clean. This is not the same question as
/// `Error::Fn` - a failure marker is not a compiler error and the two tallies
/// are reported separately.
pub(crate) fn Fn(Line:&str) -> bool {
	let t = Line.trim_start();
	t.contains("FAILED") || t.starts_with("FAIL ") || t.starts_with("--- FAIL:") || Casefold::Fn(t, "failed") >= 1
}
