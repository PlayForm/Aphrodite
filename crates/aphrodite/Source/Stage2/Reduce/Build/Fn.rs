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

	let HasSummaries = !Summaries.IsEmpty();
	let HasErrors = !Errors.IsEmpty();
	let HasWarnings = !Warnings.IsEmpty();

	// One entry per group: its header and the lines it kept. A group that kept nothing has no
	// entry, so the table decides what the reduction holds instead of three tests deciding it.
	let Parts:Vec<String> = [
		HasSummaries.then(|| ("[build summary]".to_string(), Summaries.IntoSample())),
		HasErrors.then(|| (format!("[{} errors]", Errors.Total()), Errors.IntoSample())),
		HasWarnings.then(|| (format!("[{} warnings]", Warnings.Total()), Warnings.IntoSample())),
	]
	.into_iter()
	.flatten()
	.flat_map(|(Header, Lines)| std::iter::once(Header).chain(Lines))
	.collect();

	match Parts.as_slice() {
		[] => None,
		_ => Some(Parts.join("\n")),
	}
}
