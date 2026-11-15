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

	Inp.raw.lines().for_each(|Line| {
		match Line.split("test result:").nth(1) {
			Some(Rest) => {
				Found = true;
				Pass += Exact::Fn(Rest, "passed");
				Fail += Exact::Fn(Rest, "failed");
				Ignored += Exact::Fn(Rest, "ignored");
			},
			None => {},
		}
	});

	// pytest: `=== 3 failed, 220 passed in 0.31s ===` / `220 passed`
	match !Found {
		true => {
			Inp.raw.lines().for_each(|Line| {
				match Line.contains("passed") || Line.contains("failed") {
					true => {
						let p = Exact::Fn(Line, "passed");
						let f = Exact::Fn(Line, "failed");

						match p > 0 || f > 0 {
							true => {
								Found = true;
								Pass += p;
								Fail += f;
							},
							false => {},
						}
					},
					false => {},
				}
			});
		},
		false => {},
	}

	// Bare test log without a summary line (residual #4, `term_plain`):
	// `test foo ... ok` / `... FAILED` / `... ignored` lines still give an
	// honest pass/fail tally instead of `[test:3L]`.
	match !Found {
		true => {
			Inp.raw.lines().for_each(|Line| {
				let t = Line.trim();

				match t.strip_prefix("test ") {
					Some(Rest) => {
						match () {
							_ if Rest.contains("... ok") => {
								Pass += 1;
								Found = true;
							},
							_ if Rest.contains("... FAILED") => {
								Fail += 1;
								Found = true;
							},
							_ if Rest.contains("... ignored") => {
								Ignored += 1;
								Found = true;
							},
							_ => {},
						}
					},
					None => {},
				}
			});
		},
		false => {},
	}
	// First failing test name (cargo `test NAME ... FAILED` / go `--- FAIL: NAME`).
	let FirstFail = Inp
		.raw
		.lines()
		.find_map(|l| {
			let t = l.trim();

			match () {
				_ if t.starts_with("--- FAIL: ") => {
					t.strip_prefix("--- FAIL: ")
						.map(|Rest| Rest.split_whitespace().next().unwrap_or("").to_string())
				},
				_ if t.starts_with("test ") && t.ends_with("FAILED") => {
					t.strip_prefix("test ")
						.and_then(|r| r.split_whitespace().next())
						.map(|s| s.to_string())
				},
				_ if t.starts_with("FAILED ") => {
					t.strip_prefix("FAILED ")
						.map(|r| r.split_whitespace().next().unwrap_or("").to_string())
				},
				_ => None,
			}
		})
		.filter(|s| !s.is_empty());
	// Duration if present (Phase 4: token scan replacing the old DUR_RE).
	let Dur = FirstDuration(Inp.raw);

	match !Found && FirstFail.is_none() {
		true => {
			return format!("[test:{}L]", Inp.Total);
		},
		false => {},
	}

	let mut s = format!("[test:{} pass {} fail {} ignored", Pass, Fail, Ignored);

	match FirstFail {
		Some(f) => {
			s.push_str(&format!(" | FAIL {}", f.chars().take(40).collect::<String>()));
		},
		None => {
			match Dur {
				Some(d) => {
					s.push_str(&format!(" | {}", d));
				},
				None => {},
			}
		},
	}

	s.push(']');
	s
}

/// First duration token on a line (`0.31s` / `150ms`), first match wins -
/// the same order as the old `DUR_RE` captures. A whitespace token ending
/// `s` with a `digits '.' digits` prefix, or ending `Ms` with an all-digit
/// prefix.
fn FirstDuration(Content:&str) -> Option<String> {
	Content.lines().flat_map(|Line| Line.split_whitespace()).find_map(|Tok| {
		match Tok.strip_suffix("ms") {
			Some(Ms) => (!Ms.is_empty() && Ms.chars().all(|c| c.is_ascii_digit())).then(|| format!("{Ms}ms")),
			None => {
				Tok.strip_suffix('s').and_then(|s| {
					let (a, b) = s.split_once('.')?;
					let Numeric = !a.is_empty()
						&& !b.is_empty()
						&& a.chars().all(|c| c.is_ascii_digit())
						&& b.chars().all(|c| c.is_ascii_digit());

					Numeric.then(|| format!("{s}s"))
				})
			},
		}
	})
}
