//! Python source: functions, classes, imports and decorators.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// Language, the definitions, then the line count.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Content = Analysis.Content;
	let Fns:Vec<&str> = Content
		.lines()
		.filter(|Line| {
			let Trimmed = Line.trim_start();

			Trimmed.starts_with("def ") || Trimmed.starts_with("async def ")
		})
		.filter_map(|Line| {
			let Trimmed = Line.trim_start();
			let After = Trimmed.strip_prefix("async def ").or_else(|| Trimmed.strip_prefix("def "))?;

			After.split(['(', ' ', ':']).next().filter(|Name| !Name.is_empty())
		})
		.collect();
	let Classes:Vec<&str> = Content
		.lines()
		.filter(|Line| Line.trim_start().starts_with("class "))
		.filter_map(|Line| {
			let After = Line.trim_start().strip_prefix("class ")?;

			After.split(['(', ' ', ':']).next().filter(|Name| !Name.is_empty())
		})
		.collect();
	let Imports:Vec<&str> = Content
		.lines()
		.filter(|Line| {
			let Trimmed = Line.trim_start();

			Trimmed.starts_with("import ") || Trimmed.starts_with("from ")
		})
		.filter_map(|Line| {
			let Trimmed = Line.trim_start();

			let After = Trimmed.strip_prefix("import ").or_else(|| Trimmed.strip_prefix("from "))?;

			After.split([' ', ',', ';']).next().filter(|Name| !Name.is_empty())
		})
		.collect();
	let Decorators:Vec<&str> = Content
		.lines()
		.filter(|Line| Line.trim_start().starts_with('@'))
		.filter_map(|Line| {
			let Trimmed = Line.trim_start();

			Trimmed
				.strip_prefix('@')
				.and_then(|Name| Name.split(['(', ' ']).next())
				.filter(|Name| !Name.is_empty())
		})
		.collect();

	[
		Some(MetadataField::New("lang", "py")),
		(!Fns.is_empty()).then(|| MetadataField::New("fns", Fns.join(","))),
		(!Classes.is_empty()).then(|| MetadataField::New("classes", Classes.join(","))),
		(!Imports.is_empty()).then(|| MetadataField::New("imports", Imports.join(","))),
		(!Decorators.is_empty()).then(|| MetadataField::New("decorators", Decorators.join(","))),
		Some(MetadataField::New("ln", Analysis.Lines.to_string())),
	]
	.into_iter()
	.flatten()
	.collect()
}
