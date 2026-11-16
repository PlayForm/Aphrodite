//! The build/log reducer.
//!
//! Build and log output is mostly repetition: the same "Compiling ..." / "Finished ..." chatter
//! around a handful of lines that matter. The reduction classifies each line once and emits the
//! three groups with their counts, each group bounded so a pathological log cannot produce a
//! "reduced" output longer than the input.

use crate::Stage2::Reduce::{
	Build::Classify::{Enum::Enum as Class, Fn::Fn as Classify},
	Sample::Struct::Struct as Sample,
};

/// Summary lines the reduction shows: ten.
const SummariesShown:usize = 10;

/// Error lines the reduction shows: twenty.
const ErrorsShown:usize = 20;

/// Warning lines the reduction shows: ten.
const WarningsShown:usize = 10;

/// Characters of one retained line: two hundred.
const LineShown:usize = 200;

/// Reduce build/log output: extract errors, warnings, summaries.
///
/// Each group keeps an exact total and the first N of its lines, so `[100 errors]` stays true
/// while the body carries twenty - the previous implementation collected all hundred and then cut
/// the list, which made a long log cost memory proportional to its own length for no extra output.
///
/// It is not a compiler: nothing is resolved, and a line that starts with `error:` is counted by
/// the summary group rather than the error group.
pub fn ReduceBuild(Content:&str) -> Option<String> {
	let mut Summaries = Sample::<String, SummariesShown>::new();
	let mut Errors = Sample::<String, ErrorsShown>::new();
	let mut Warnings = Sample::<String, WarningsShown>::new();

	Content.lines().filter(|Line| !Line.trim().is_empty()).for_each(|Line| {
		let Retained = || Line.trim().chars().take(LineShown).collect::<String>();

		match Classify(Line) {
			Class::Error => Errors.Push(Retained),
			Class::Warning => Warnings.Push(Retained),
			Class::Summary => Summaries.Push(Retained),
			Class::Ignore => {},
		}
	});

	let mut Parts:Vec<String> = Vec::new();

	if !Summaries.IsEmpty() {
		Parts.push("[build summary]".into());
		Parts.extend(Summaries.IntoSample());
	}

	if !Errors.IsEmpty() {
		Parts.push(format!("[{} errors]", Errors.Total()));
		Parts.extend(Errors.IntoSample());
	}

	if !Warnings.IsEmpty() {
		Parts.push(format!("[{} warnings]", Warnings.Total()));
		Parts.extend(Warnings.IntoSample());
	}

	match Parts.as_slice() {
		[] => None,
		_ => Some(Parts.join("\n")),
	}
}
