//! Test-output shape detector.
//!
//! A `test result:` line is unambiguous, but a bare test log has no summary - it
//! has a `running N tests` header and per-test result lines. Both shapes are
//! recognised here, so a run without a summary still previews as a test run. It
//! is not a verdict: nothing is executed and no result is inferred.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Line::Test::{NumberAfter, NumberBefore, Result, Running},
};

/// True when the content looks like test output.
///
/// A `test result:` / `N passed` / `=== RUN` / pytest summary line is a strong
/// signal even amid other noise. A bare log also counts when it carries both a
/// `running N tests` header and `test X ... ok` result lines. It is not a
/// pass/fail check - the arm that renders the tally decides that.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	Inp.raw.contains("test result:")
		|| Inp.raw.contains("=== RUN ")
		|| Inp.raw.contains("--- FAIL:")
		|| Inp.raw.contains("--- PASS:")
		|| Inp
			.raw
			.lines()
			.any(|l| NumberBefore::Fn(l, "passed") || NumberBefore::Fn(l, "failed"))
		|| Inp.raw.lines().any(|l| NumberAfter::Fn(l, "Tests:"))
		|| Inp.raw.lines().any(|l| Running::Fn(l.trim_start()))
		|| Inp.raw.lines().any(|l| Result::Fn(l.trim_start()))
}
