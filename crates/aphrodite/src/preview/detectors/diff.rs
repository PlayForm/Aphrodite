//! diff shape detector.

use crate::preview::input::Input;

/// git/unified headers or a hunk: `diff --git `, a `--- `/`+++ ` pair, or an
/// `@@ ` hunk with ≥1 real delta line (`+`/`-` content, excluding the `+++`/
/// `---` header lines themselves).
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	let has_diff_git = inp.non_empty.iter().any(|l| l.trim_start().starts_with("diff --git "));
	let has_ab_headers = {
		let a = inp.non_empty.iter().any(|l| l.trim_start().starts_with("--- "));
		let b = inp.non_empty.iter().any(|l| l.trim_start().starts_with("+++ "));
		a && b
	};
	let has_hunk = {
		let h = inp.non_empty.iter().any(|l| l.trim_start().starts_with("@@ "));
		let delta = inp
			.non_empty
			.iter()
			.filter(|l| {
				let t = l.trim_start();
				(t.starts_with('+') && !t.starts_with("+++")) || (t.starts_with('-') && !t.starts_with("---"))
			})
			.count();
		h && delta >= 1
	};
	has_diff_git || has_ab_headers || has_hunk
}