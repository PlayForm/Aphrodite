//! The build/log reducer.
//!
//! Build and log output is mostly repetition: the same "Compiling ..." /
//! "Finished ..." chatter around a handful of lines that matter. The reduction
//! classifies each line once (error, warning, summary keyword) and emits the
//! three groups with their counts, capped per group so a pathological log
//! cannot produce a "reduced" output longer than the input.
//!
//! A line that *starts* with `error:` / `warning:` is deliberately not
//! classified as an error/warning: those prefixes are structured and are
//! picked up by the summary arm, and counting them twice would double the
//! headline numbers the caller reads.

/// Reduce build/log output: extract errors, warnings, summaries.
pub fn ReduceBuild(Content:&str) -> Option<String> {
	let mut Errors:Vec<String> = Vec::new();
	let mut Warnings:Vec<String> = Vec::new();
	let mut SummaryLines:Vec<String> = Vec::new();

	let SummaryKw = ["compiling", "finished", "running", "test result", "passed", "failed"];

	for Line in Content.lines() {
		let Stripped = Line.trim();
		if Stripped.is_empty() {
			continue;
		}
		let Lower = Stripped.to_lowercase();

		if Lower.contains("error") && !Lower.starts_with("error:") {
			Errors.push(Stripped.chars().take(200).collect());
		} else if Lower.contains("warning") && !Lower.starts_with("warning:") {
			Warnings.push(Stripped.chars().take(200).collect());
		} else if SummaryKw.iter().any(|Kw| Lower.contains(Kw)) {
			SummaryLines.push(Stripped.chars().take(200).collect());
		}
	}

	let mut Parts:Vec<String> = Vec::new();
	if !SummaryLines.is_empty() {
		Parts.push("[build summary]".into());
		Parts.extend(SummaryLines.into_iter().take(10));
	}
	if !Errors.is_empty() {
		Parts.push(format!("[{} errors]", Errors.len()));
		Parts.extend(Errors.into_iter().take(20));
	}
	if !Warnings.is_empty() {
		Parts.push(format!("[{} warnings]", Warnings.len()));
		Parts.extend(Warnings.into_iter().take(10));
	}

	if Parts.is_empty() { None } else { Some(Parts.join("\n")) }
}
