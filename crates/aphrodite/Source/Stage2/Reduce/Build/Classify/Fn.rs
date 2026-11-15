//! Classify one build-log line.

use crate::Stage2::Reduce::Build::Classify::Enum;

/// Lowercase substrings that mark a progress/summary line.
const SummaryKeywords:[&str; 6] = ["compiling", "finished", "running", "test result", "passed", "failed"];

/// Classify a build-log line as an error, a warning, a summary line, or noise.
///
/// A line that STARTS with `error:` or `warning:` is deliberately not classified as one: those
/// prefixes are structured and are picked up by the summary arm, and counting them twice would
/// double the headline numbers the caller reads. The three tests are tried in that order, so a
/// line that mentions both `error` and `warning` counts as an error exactly once.
///
/// It is not a diagnostic parser: the class is decided by substrings of the lowercased line, which
/// is what the previous guard chain did.
pub(crate) fn Fn(Line:&str) -> Enum::Enum {
	let Lower = Line.trim().to_lowercase();
	let Error = Lower.contains("error") && !Lower.starts_with("error:");
	let Warning = Lower.contains("warning") && !Lower.starts_with("warning:");
	let Summary = SummaryKeywords.iter().any(|Keyword| Lower.contains(Keyword));

	Error
		.then_some(Enum::Enum::Error)
		.or_else(|| Warning.then_some(Enum::Enum::Warning))
		.or_else(|| Summary.then_some(Enum::Enum::Summary))
		.unwrap_or(Enum::Enum::Ignore)
}
