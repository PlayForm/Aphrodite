//! `Extract::Diff::Fn`: the `+`/`-` line tallies.

/// `+`/`-` line tallies for diff content (excluding `+++`/`---` hunk
/// headers); `("?", "?")` when the content has no diff lines at all.
///
/// The `+++`/`---` exclusion is not cosmetic: a unified diff carries exactly
/// one of each as its file headers, so counting them adds a phantom added and
/// removed line to every single diff. The `("?", "?")` default distinguishes
/// "not a diff" from "an empty diff" (`0`/`0`).
pub(crate) fn Fn(Lines:&[&str]) -> (String, String) {
	let Plus = Lines.iter().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
	let Minus = Lines.iter().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
	match Plus == 0 && Minus == 0 {
		true => ("?".into(), "?".into()),
		false => (Plus.to_string(), Minus.to_string()),
	}
}
