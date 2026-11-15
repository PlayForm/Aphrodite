//! JavaScript source: `function` and `const` bindings.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// Language, the binding names, then the line count.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Fns:Vec<&str> = Analysis
		.Content
		.lines()
		.filter(|Line| {
			let Trimmed = Line.trim_start();

			Trimmed.starts_with("function ") || Trimmed.starts_with("const ")
		})
		.filter_map(|Line| {
			let Trimmed = Line.trim_start();

			Trimmed
				.strip_prefix("function ")
				.or_else(|| Trimmed.strip_prefix("const "))
				.and_then(|After| After.split(['(', ' ', '=', ':']).next())
				.filter(|Name| !Name.is_empty())
		})
		.collect();

	[
		Some(MetadataField::New("lang", "js")),
		(!Fns.is_empty()).then(|| MetadataField::New("fns", Fns.join(","))),
		Some(MetadataField::New("ln", Analysis.Lines.to_string())),
	]
	.into_iter()
	.flatten()
	.collect()
}
