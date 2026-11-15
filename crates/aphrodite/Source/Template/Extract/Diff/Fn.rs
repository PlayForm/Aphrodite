//! The `+`/`-` line tallies.

use super::Struct::Struct;

/// `+`/`-` line tallies for diff content, or `None` when the content has no diff
/// lines at all.
///
/// The `+++`/`---` exclusion is not cosmetic: a unified diff carries exactly one of
/// each as its file headers, so counting them adds a phantom added and removed line
/// to every single diff.
///
/// The absence answer is `None` rather than `0`/`0`, because "not a diff" and "a
/// diff that changed nothing" are different facts and the renderer spells them
/// differently (`?`/`?` versus `0`/`0`).
pub(crate) fn Fn(Lines:&[&str]) -> Option<Struct> {
	let Plus = Lines
		.iter()
		.filter(|Line| Line.starts_with('+') && !Line.starts_with("+++"))
		.count();
	let Minus = Lines
		.iter()
		.filter(|Line| Line.starts_with('-') && !Line.starts_with("---"))
		.count();

	(Plus + Minus > 0).then_some(Struct { Plus, Minus })
}
