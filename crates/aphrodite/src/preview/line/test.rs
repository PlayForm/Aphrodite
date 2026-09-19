//! Test-output line predicates: `running N tests` headers, `test X ... ok`
//! result lines, and `N passed` / `Tests: N` number probes.

/// `running N tests` header (bare test logs without a summary line).
pub(crate) fn is_running_tests_line(t:&str) -> bool {
	let rest = match t.strip_prefix("running ") {
		Some(r) => r,
		None => return false,
	};
	let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
	if digits == 0 {
		return false;
	}
	let after = rest[digits..].trim_start();
	let after = match after.strip_prefix("test") {
		Some(a) => a,
		None => return false,
	};
	let after = match after.strip_prefix('s') {
		Some(a) => a,
		None => after,
	};
	// `tests?\b`: the word must end here or be followed by a non-word char.
	after.chars().next().map(|c| !(c.is_alphanumeric() || c == '_')).unwrap_or(true)
}

/// `test <name> ... ok|FAILED|ignored` line.
pub(crate) fn is_test_result_line(t:&str) -> bool {
	let rest = match t.strip_prefix("test ") {
		Some(r) => r,
		None => return false,
	};
	let name_len = rest.chars().take_while(|c| !c.is_whitespace()).count();
	if name_len == 0 {
		return false;
	}
	let after = rest[name_len..].trim_start();
	let after = match after.strip_prefix("...") {
		Some(a) => a,
		None => return false,
	};
	let after = after.trim_start();
	for kw in ["ok", "FAILED", "ignored"] {
		if let Some(r) = after.strip_prefix(kw) {
			// word boundary: next char is end or non-word.
			return r.chars().next().map(|c| !(c.is_alphanumeric() || c == '_')).unwrap_or(true);
		}
	}
	false
}

/// `N passed` / `N failed` anywhere on a line (pytest summaries).
pub(crate) fn has_number_before(line:&str, kw:&str) -> bool {
	match line.find(kw) {
		Some(i) => line[..i].trim_end().chars().last().map(|c| c.is_ascii_digit()).unwrap_or(false),
		None => false,
	}
}

/// `Tests: N` jest summary line.
pub(crate) fn has_number_after(line:&str, kw:&str) -> bool {
	match line.find(kw) {
		Some(i) => {
			line[i + kw.len()..]
				.trim_start()
				.chars()
				.next()
				.map(|c| c.is_ascii_digit())
				.unwrap_or(false)
		},
		None => false,
	}
}