//! test-output preview arm.

use crate::preview::input::Input;
use crate::preview::text::num_before::num_before;

/// test-output preview: pass/fail/ignored tallies + first failing test.
/// `[test:220 pass 0 fail 1 ignored | 0.31s]` / names the first failure.
pub(crate) fn build_test_preview(inp:&Input<'_>) -> String {
	// cargo: `test result: ok. 220 passed; 0 failed; 1 ignored; ... 0.31s`
	let mut pass = 0usize;
	let mut fail = 0usize;
	let mut ignored = 0usize;
	let mut found = false;
	for line in inp.raw.lines() {
		if let Some(rest) = line.split("test result:").nth(1) {
			found = true;
			pass += num_before(rest, "passed");
			fail += num_before(rest, "failed");
			ignored += num_before(rest, "ignored");
		}
	}
	// pytest: `=== 3 failed, 220 passed in 0.31s ===` / `220 passed`
	if !found {
		for line in inp.raw.lines() {
			if line.contains("passed") || line.contains("failed") {
				let p = num_before(line, "passed");
				let f = num_before(line, "failed");
				if p > 0 || f > 0 {
					found = true;
					pass += p;
					fail += f;
				}
			}
		}
	}
	// Bare test log without a summary line (residual #4, `term_plain`):
	// `test foo ... ok` / `... FAILED` / `... ignored` lines still give an
	// honest pass/fail tally instead of `[test:3L]`.
	if !found {
		for line in inp.raw.lines() {
			let t = line.trim();
			if let Some(rest) = t.strip_prefix("test ") {
				if rest.contains("... ok") {
					pass += 1;
					found = true;
				} else if rest.contains("... FAILED") {
					fail += 1;
					found = true;
				} else if rest.contains("... ignored") {
					ignored += 1;
					found = true;
				}
			}
		}
	}
	// First failing test name (cargo `test NAME ... FAILED` / go `--- FAIL: NAME`).
	let first_fail = inp
		.raw
		.lines()
		.find_map(|l| {
			let t = l.trim();
			if let Some(rest) = t.strip_prefix("--- FAIL: ") {
				Some(rest.split_whitespace().next().unwrap_or("").to_string())
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
	let dur = first_duration(inp.raw);

	if !found && first_fail.is_none() {
		return format!("[test:{}L]", inp.total);
	}
	let mut s = format!("[test:{} pass {} fail {} ignored", pass, fail, ignored);
	if let Some(f) = first_fail {
		s.push_str(&format!(" | FAIL {}", f.chars().take(40).collect::<String>()));
	} else if let Some(d) = dur {
		s.push_str(&format!(" | {}", d));
	}
	s.push(']');
	s
}

/// First duration token on a line (`0.31s` / `150ms`), first match wins -
/// the same order as the old `DUR_RE` captures. A whitespace token ending
/// `s` with a `digits '.' digits` prefix, or ending `ms` with an all-digit
/// prefix.
fn first_duration(content:&str) -> Option<String> {
	for line in content.lines() {
		for tok in line.split_whitespace() {
			if let Some(ms) = tok.strip_suffix("ms") {
				if !ms.is_empty() && ms.chars().all(|c| c.is_ascii_digit()) {
					return Some(format!("{ms}ms"));
				}
			} else if let Some(s) = tok.strip_suffix('s') {
				if let Some((a, b)) = s.split_once('.') {
					if !a.is_empty()
						&& !b.is_empty()
						&& a.chars().all(|c| c.is_ascii_digit())
						&& b.chars().all(|c| c.is_ascii_digit())
					{
						return Some(format!("{s}s"));
					}
				}
			}
		}
	}
	None
}