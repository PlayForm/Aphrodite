//! Diff shape detector.
//!
//! A diff is recognised by its headers or by a hunk with real deltas. This
//! detector requires one of those. It is not a patch validator: an unapplied or
//! malformed diff still counts, because the preview only has to describe it.

use crate::Preview::Input::Struct::Struct as Input;

/// True when the content looks like a diff.
///
/// Git/unified headers (`diff --git `, a `--- `/`+++ ` pair) or an `@@ ` hunk with
/// at least one real delta line (`+`/`-` content, excluding the header lines
/// themselves). It is not a change summary: nothing is applied and no file is
/// read.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let Git = Inp.NonEmpty.iter().any(|l| l.trim_start().starts_with("diff --git "));
	let Unified = {
		let a = Inp.NonEmpty.iter().any(|l| l.trim_start().starts_with("--- "));
		let b = Inp.NonEmpty.iter().any(|l| l.trim_start().starts_with("+++ "));
		a && b
	};
	let Hunk = {
		let h = Inp.NonEmpty.iter().any(|l| l.trim_start().starts_with("@@ "));
		let Delta = Inp
			.NonEmpty
			.iter()
			.filter(|l| {
				let t = l.trim_start();
				(t.starts_with('+') && !t.starts_with("+++")) || (t.starts_with('-') && !t.starts_with("---"))
			})
			.count();
		h && Delta >= 1
	};
	Git || Unified || Hunk
}
