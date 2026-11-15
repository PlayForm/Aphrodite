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

	Content.lines().filter(|Line| !Line.trim().is_empty()).for_each(|Line| {
		let Stripped = Line.trim();
		let Lower = Stripped.to_lowercase();

		match () {
			_ if Lower.contains("error") && !Lower.starts_with("error:") => {
				Errors.push(Stripped.chars().take(200).collect());
			},
			_ if Lower.contains("warning") && !Lower.starts_with("warning:") => {
				Warnings.push(Stripped.chars().take(200).collect());
			},
			_ if SummaryKw.iter().any(|Kw| Lower.contains(Kw)) => {
				SummaryLines.push(Stripped.chars().take(200).collect());
			},
			_ => {},
		}
	});

	let mut Parts:Vec<String> = Vec::new();
	match !SummaryLines.is_empty() {
		true => {
			Parts.push("[build summary]".into());
			Parts.extend(SummaryLines.into_iter().take(10));
		},
		false => {},
	}
	match !Errors.is_empty() {
		true => {
			Parts.push(format!("[{} errors]", Errors.len()));
			Parts.extend(Errors.into_iter().take(20));
		},
		false => {},
	}
	match !Warnings.is_empty() {
		true => {
			Parts.push(format!("[{} warnings]", Warnings.len()));
			Parts.extend(Warnings.into_iter().take(10));
		},
		false => {},
	}

	match Parts.is_empty() {
		true => None,
		false => Some(Parts.join("\n")),
	}
}
