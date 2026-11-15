//! Generate the metadata block that accompanies a compressed marker.
//!
//! Metadata is the model's map of what it is no longer holding: the content
//! family, its size, its shape and the reason it was compressed. It is built per
//! content family because the useful facts differ - a build log's value is its
//! error lines, a JSON payload's is its key structure. A misleading block is a
//! bug, not cosmetic.
//!
//! Every producer returns typed fields in the order the model should read them;
//! the family is a `ContentType::Family` value, not a string comparison, so the
//! threshold scaler and this renderer cannot disagree about what a name means.

use crate::Proxy::{
	Analysis::Struct::Struct as ContentAnalysis,
	ContentType::Family::{Classify::Fn::Fn as ClassifyFamily, Enum::Enum as ContentFamily},
	Metadata::Field::Struct::Struct as MetadataField,
};
use super::{Build, Code, Diff, Error, Git, Go, JS, JSON, Linter, Log, Python, Rust, Text};

/// The marker's metadata block: `;`-separated `key=value`, at most 400 chars.
pub(crate) fn Fn(Analysis:&ContentAnalysis<'_>) -> String {
	let Parts:Vec<MetadataField> = match ClassifyFamily(Analysis.Ct) {
		ContentFamily::CodeRust => Rust::Fn::Fn(Analysis),
		ContentFamily::CodePython => Python::Fn::Fn(Analysis),
		ContentFamily::CodeGo => Go::Fn::Fn(Analysis),
		ContentFamily::CodeJS => JS::Fn::Fn(Analysis),
		ContentFamily::Code => Code::Fn::Fn(Analysis),
		ContentFamily::Error => Error::Fn::Fn(Analysis),
		ContentFamily::Diff => Diff::Fn::Fn(Analysis),
		ContentFamily::Git => Git::Fn::Fn(Analysis),
		ContentFamily::BuildOutput => Build::Fn::Fn(Analysis),
		ContentFamily::Log => Log::Fn::Fn(Analysis),
		ContentFamily::Linter => Linter::Fn::Fn(Analysis),
		ContentFamily::Json | ContentFamily::ToolOutput => JSON::Fn::Fn(Analysis),
		ContentFamily::Text | ContentFamily::Other => Text::Fn::Fn(Analysis),
	};

	// `;` separates fields and is safe inside the marker's `|` delimiters;
	// commas separate list items within a value. 400 chars is coding-tuned:
	// enough for ~25 function names.
	let Joined = Parts.iter().map(MetadataField::Render).collect::<Vec<_>>().join(";");
	let Truncated:String = Joined.replace('\n', " ").replace('\r', "").chars().take(400).collect();

	Truncated.trim_end_matches([';', ' ', ',']).to_string()
}
