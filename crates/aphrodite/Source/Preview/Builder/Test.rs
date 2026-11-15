//! Test-output preview arm.
//!
//! A test run is summarised by three numbers - passed, failed, ignored - and by
//! the name of the first failure. This arm renders those. It is not a test
//! runner: it reads the text the runner printed, so a runner with an unfamiliar
//! summary shape previews with zeroes rather than with a guess.

use crate::Preview::{Input::Struct::Struct as Input, Text::NumberBefore::Exact};

/// Build a preview of test output.
///
/// Promises pass/fail/ignored tallies plus the first failing test's name and the
/// run duration. It is not a verdict: a run whose summary line is missing reports
/// no tally, and the failing-test name is shown so that a run which failed
/// without a summary can never look clean.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	// cargo: `test result: ok. 220 passed; 0 failed; 1 ignored; ... 0.31s`
	let mut Pass = 0usize;
	let mut Fail = 0usize;
	let mut Ignored = 0usize;
	let mut Found = false;
	for Line in Inp.raw.lines() {
		if let Some(Rest) = Line.split("test result:").nth(1) {
			Found = true;
			Pass += Exact::Fn(Rest, "passed");
			Fail += Exact::Fn(Rest, "failed");
			Ignored += Exact::Fn(Rest, "ignored");
		}
	}
	// pytest: `=== 3 failed, 220 passed in 0.31s ===` / `220 passed`
	if !Found {
		for Line in Inp.raw.lines() {
			if Line.contains("passed") || Line.contains("failed") {
				let p = Exact::Fn(Line, "passed");
				let f = Exact::Fn(Line, "failed");
				if p > 0 || f > 0 {
					Found = true;
					Pass += p;
					Fail += f;
				}
			}
		}
	}
	// Bare test log without a summary line (residual #4, `term_plain`):
	// `test foo ... ok` / `... FAILED` / `... ignored` lines still give an
	// honest pass/fail tally instead of `[test:3L]`.
	if !Found {
		for Line in Inp.raw.lines() {
			let t = Line.trim();
			if let Some(Rest) = t.strip_prefix("test ") {
				if Rest.contains("... ok") {
					Pass += 1;
					Found = true;
				} else if Rest.contains("... FAILED") {
					Fail += 1;
					Found = true;
				} else if Rest.contains("... ignored") {
					Ignored += 1;
					Found = true;
				}
			}
		}
	}
	// First failing test name (cargo `test NAME ... FAILED` / go `--- FAIL: NAME`).
	let FirstFail = Inp
		.raw
		.lines()
		.find_map(|l| {
			let t = l.trim();
			if let Some(Rest) = t.strip_prefix("--- FAIL: ") {
				Some(Rest.split_whitespace().next().unwrap_or("").to_string())
			} else if t.starts_with("test ") && t.ends_with("FAILED") {
				t.strip_prefix("test ")
					.and_then(|r| r.split_whitespace().next())
					.map(|s| s.to_string())
			} else if t.starts_with("FAILED ") {
				t.strip_prefix("FAILED ")
					.map(|r| r.split_whitespace().next().unwrap_or("").to_string())
			} else {
				None
			}
		})
		.filter(|s| !s.is_empty());
	// Duration if present (Phase 4: token scan replacing the old DUR_RE).
	let Dur = FirstDuration(Inp.raw);

	if !Found && FirstFail.is_none() {
		return format!("[test:{}L]", Inp.Total);
	}
	let mut s = format!("[test:{} pass {} fail {} ignored", Pass, Fail, Ignored);
	if let Some(f) = FirstFail {
		s.push_str(&format!(" | FAIL {}", f.chars().take(40).collect::<String>()));
	} else if let Some(d) = Dur {
		s.push_str(&format!(" | {}", d));
	}
	s.push(']');
	s
}

/// First duration token on a line (`0.31s` / `150ms`), first match wins -
/// the same order as the old `DUR_RE` captures. A whitespace token ending
/// `s` with a `digits '.' digits` prefix, or ending `Ms` with an all-digit
/// prefix.
fn FirstDuration(Content:&str) -> Option<String> {
	for Line in Content.lines() {
		for Tok in Line.split_whitespace() {
			if let Some(Ms) = Tok.strip_suffix("ms") {
				if !Ms.is_empty() && Ms.chars().all(|c| c.is_ascii_digit()) {
					return Some(format!("{Ms}ms"));
				}
			} else if let Some(s) = Tok.strip_suffix('s')
				&& let Some((a, b)) = s.split_once('.')
				&& !a.is_empty()
				&& !b.is_empty()
				&& a.chars().all(|c| c.is_ascii_digit())
				&& b.chars().all(|c| c.is_ascii_digit())
			{
				return Some(format!("{s}s"));
			}
		}
	}
	None
}
