//! FAILED-state line predicate (failure signals, honest `N failed` counts).

use crate::preview::text::num_before::num_before_kw;

/// True for a line that signals a FAILED state: a `FAILED`/`FAIL` marker, a
/// `--- FAIL:` test failure, or a NON-ZERO `N failed` count. `0 failed`
/// (clean runs) never matches, so a passing test summary stays clean.
pub(crate) fn is_failure_line(line:&str) -> bool {
	let t = line.trim_start();
	t.contains("FAILED") || t.starts_with("FAIL ") || t.starts_with("--- FAIL:") || num_before_kw(t, "failed") >= 1
}