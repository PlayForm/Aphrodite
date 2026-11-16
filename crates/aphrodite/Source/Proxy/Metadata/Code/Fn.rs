//! Code of unknown language: any signature-looking line, labelled `gen`.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// Language, then the signatures found, then the line count.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Sigs:Vec<&str> = Analysis
		.Content()
		.lines()
		.filter(|Line| {
			let Trimmed = Line.trim_start();

			Trimmed.starts_with("fn ")
				|| Trimmed.starts_with("def ")
				|| Trimmed.starts_with("func ")
				|| Trimmed.starts_with("function ")
				|| Trimmed.starts_with("class ")
				|| Trimmed.starts_with("struct ")
		})
		.filter_map(|Line| {
			let Trimmed = Line.trim_start();
			let After = Trimmed
				.strip_prefix("fn ")
				.or_else(|| Trimmed.strip_prefix("def "))
				.or_else(|| Trimmed.strip_prefix("func "))
				.or_else(|| Trimmed.strip_prefix("function "))
				.or_else(|| Trimmed.strip_prefix("class "))
				.or_else(|| Trimmed.strip_prefix("struct "))?;

			After.split(['(', ' ']).next()
		})
		.collect();

	[
		Some(MetadataField::New("lang", "gen")),
		(!Sigs.is_empty()).then(|| MetadataField::New("sigs", Sigs.join(","))),
		Some(MetadataField::New("ln", Analysis.Lines().to_string())),
	]
	.into_iter()
	.flatten()
	.collect()
}
