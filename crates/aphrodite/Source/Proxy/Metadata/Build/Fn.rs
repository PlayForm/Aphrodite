//! Build output: OK/FAIL, the files compiled, the first error.

use crate::Proxy::{Analysis::Struct::Struct as ContentAnalysis, Metadata::Field::Struct::Struct as MetadataField};

/// The markers that make a build log a failure.
const FailureMarkers:[&str; 2] = ["error", "aborting"];

/// Status first, then how much was built, then the error that stopped it.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> Vec<MetadataField> {
	let Content = Analysis.Content();
	// A table of failure markers rather than a chain of tests: the first marker
	// the content carries names the status, and no marker means it was OK.
	let Status = match FailureMarkers.iter().find(|Marker| Content.contains(**Marker)) {
		Some(_) => "FAIL",
		None => "OK",
	};
	let Files = Content
		.lines()
		.filter(|Line| Line.starts_with("Compiling ") || Line.contains(" Compiling "))
		.count();
	let FirstError = Content
		.lines()
		.find(|Line| Line.contains("error[") || Line.contains("Error:"))
		.map(|Line| Line.trim().chars().take(80).collect::<String>().replace('|', "/"));

	[
		Some(MetadataField::New("status", Status)),
		(Files > 0).then(|| MetadataField::New("files", Files.to_string())),
		FirstError.map(|Error| MetadataField::New("first_err", Error)),
	]
	.into_iter()
	.flatten()
	.collect()
}
