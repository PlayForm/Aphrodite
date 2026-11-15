//! Log output: the highest level seen and the last line.
//!
//! The level list is ordered most-severe first, and `find` takes the first name
//! a line contains, so a line carrying both `ERROR` and `WARN` reports `error` -
//! the order is the contract, not the spelling.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// The worst level in view, the last line, then the line count.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Content = Analysis.Content;
	let Level = Content.lines().find_map(|Line| {
		["ERROR", "WARN", "WARNING", "INFO", "DEBUG", "TRACE", "FATAL", "PANIC"]
			.iter()
			.find(|Level| Line.contains(**Level))
			.map(|Level| Level.to_lowercase())
	});
	let LastLine = Content.lines().last().unwrap_or("").trim().chars().take(60).collect::<String>();

	[
		Level.map(|Level| MetadataField::New("level", Level)),
		(!LastLine.is_empty()).then(|| MetadataField::New("last", LastLine.replace('|', "/"))),
		Some(MetadataField::New("ln", Analysis.Lines.to_string())),
	]
	.into_iter()
	.flatten()
	.collect()
}
