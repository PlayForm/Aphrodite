//! git-status shape detector.
//!
//! A status listing is a majority of lines carrying a porcelain code. This detector requires that
//! majority so a diff hunk or prose is not mistaken for a listing. It is not a repository query - the
//! text is all that is read.

use crate::Preview::{Input::Struct::Struct as Input, Line::Git::Status};

/// True when the content looks like `git status` output.
///
/// A majority of non-empty lines must carry a porcelain code (`M `, `A `, `D `, `R `, `??`, `UU`). The
/// long form without `--porcelain` counts too when it opens with `On branch ` or `Your branch `,
/// because the classifier recognises that shape and dropping it would regress those dumps to plain
/// text. It is not a clean check: a clean tree still previews as git output.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let OnBranch = Inp
		.raw
		.lines()
		.any(|Line| Line.trim_start().starts_with("On branch ") || Line.trim_start().starts_with("Your branch "));
	let Status = Inp.Count(|Line| Status::Fn(Line).is_some());

	OnBranch || (Status >= 2 && Status * 2 >= Inp.NonEmpty.len())
}
