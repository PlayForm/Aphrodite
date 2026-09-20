//! test-output shape detector.

use crate::preview::{
	input::Input,
	line::test::{has_number_after, has_number_before, is_running_tests_line, is_test_result_line},
};

/// cargo/pytest/jest/go: a `test result:` / `N passed` / `=== RUN` / pytest
/// summary line is a strong, unambiguous signal even amid other noise.
/// Extended for the SHALLOW tail (residual #4): `running N tests` +
/// `test X ... ok` lines (a bare test log without a summary line) also count.
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	inp.raw.contains("test result:")
		|| inp.raw.contains("=== RUN ")
		|| inp.raw.contains("--- FAIL:")
		|| inp.raw.contains("--- PASS:")
		|| inp
			.raw
			.lines()
			.any(|l| has_number_before(l, "passed") || has_number_before(l, "failed"))
		|| inp.raw.lines().any(|l| has_number_after(l, "Tests:"))
		|| inp.raw.lines().any(|l| is_running_tests_line(l.trim_start()))
		|| inp.raw.lines().any(|l| is_test_result_line(l.trim_start()))
}
