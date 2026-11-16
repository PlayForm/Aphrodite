//! Render the `git status` tally.

use std::num::NonZeroUsize;

use crate::Preview::Summary::Git::Struct::Struct as Summary;

/// Characters one path may occupy: forty.
const PathShown:usize = 40;

/// Render a status summary as `[git:<tally> | <paths> +N more]`.
///
/// The tally comes from the summary in the canonical code order, and the paths keep the
/// order they were listed in, each cut to forty characters. It is not a diff: no line
/// counts and no content, because the arm promises the shape of the change set, not the
/// change.
pub(crate) fn Fn(Summary:&Summary, Total:usize) -> String {
	let Counts:Vec<String> = Summary.Codes().map(|(Label, Count)| format!("{Count}{Label}")).collect();

	match Counts.as_slice() {
		[] => format!("[git:{Total}L]"),
		_ => {
			let Paths:Vec<String> = Summary
				.Paths()
				.iter()
				.map(|Path| Path.chars().take(PathShown).collect())
				.collect();
			let More = NonZeroUsize::new(Summary.Total().saturating_sub(Summary.Paths().len()))
				.map(|More| format!(" +{More} more"))
				.unwrap_or_default();

			match Paths.as_slice() {
				[] => format!("[git:{}]", Counts.join(" ")),
				_ => format!("[git:{} | {}{}]", Counts.join(" "), Paths.join(" "), More),
			}
		},
	}
}
