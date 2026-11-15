//! `git status` / `git log` output: the branch and the commit count.
//!
//! Two ways to find the branch, in priority order: `git status` says
//! `On branch X` outright, while a log or a status excerpt only names it on a
//! line that is neither a file entry (`* `) nor a continuation (`  `). The
//! fallback runs only when the first search found nothing, which is what
//! `or_else` guarantees.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// The branch, when one can be named, then how many commits are in view.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Content = Analysis.Content;
	let Branch = Content
		.lines()
		.map(str::trim)
		.find_map(|Line| Line.strip_prefix("On branch ").map(|Rest| Rest.trim().replace('|', "/")))
		.or_else(|| {
			Content
				.lines()
				.map(str::trim)
				.find(|Line| !Line.is_empty() && !Line.starts_with("* "))
				.map(|Line| Line.chars().take(40).collect::<String>().replace('|', "/"))
		});
	let Commits = Content
		.lines()
		.filter(|Line| Line.starts_with("commit ") || Line.trim().starts_with("* ") || Line.contains("commit"))
		.count();

	[
		Branch.map(|Branch| MetadataField::New("branch", Branch)),
		(Commits > 0).then(|| MetadataField::New("commits", Commits.to_string())),
	]
	.into_iter()
	.flatten()
	.collect()
}
