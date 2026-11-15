//! A unified diff: file count, added lines, deleted lines.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// How wide the change is, and in which direction.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Content = Analysis.Content;
	let Files = Content.lines().filter(|Line| Line.starts_with("diff --git ")).count();
	let Adds = Content
		.lines()
		.filter(|Line| Line.starts_with('+') && !Line.starts_with("+++"))
		.count();
	let Dels = Content
		.lines()
		.filter(|Line| Line.starts_with('-') && !Line.starts_with("---"))
		.count();

	[
		(Files > 0).then(|| MetadataField::New("files", Files.to_string())),
		(Adds > 0).then(|| MetadataField::New("adds", Adds.to_string())),
		(Dels > 0).then(|| MetadataField::New("dels", Dels.to_string())),
	]
	.into_iter()
	.flatten()
	.collect()
}
