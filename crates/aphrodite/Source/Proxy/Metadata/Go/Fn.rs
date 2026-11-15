//! Go source: `func` names and the line count.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// Language, the function names, then the line count.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Fns:Vec<&str> = Analysis
		.Content
		.lines()
		.filter(|Line| Line.trim_start().starts_with("func "))
		.filter_map(|Line| {
			let Trimmed = Line.trim_start();
			let After = Trimmed.strip_prefix("func ")?;

			After.split(['(', ' ']).next().filter(|Name| !Name.is_empty())
		})
		.collect();

	[
		Some(MetadataField::New("lang", "go")),
		(!Fns.is_empty()).then(|| MetadataField::New("fns", Fns.join(","))),
		Some(MetadataField::New("ln", Analysis.Lines.to_string())),
	]
	.into_iter()
	.flatten()
	.collect()
}
