//! Linter output: files with findings, the first error, the line count.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// How many files complained, the first complaint, then the line count.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Content = Analysis.Content();
	let FilesLinted = Content
		.lines()
		.filter(|Line| {
			(Line.contains(".rs:") || Line.contains(".py:") || Line.contains(".go:") || Line.contains(".ts:"))
				&& (Line.contains("error") || Line.contains("warning"))
		})
		.count();
	let FirstError = Content
		.lines()
		.find(|Line| Line.contains("error[") || Line.contains("Error:") || Line.starts_with("error: "))
		.map(|Line| Line.trim().chars().take(80).collect::<String>().replace('|', "/"));

	[
		(FilesLinted > 0).then(|| MetadataField::New("files", FilesLinted.to_string())),
		FirstError.map(|Error| MetadataField::New("first_err", Error)),
		Some(MetadataField::New("ln", Analysis.Lines().to_string())),
	]
	.into_iter()
	.flatten()
	.collect()
}
