//! The diff reducer.
//!
//! A unified diff's value to a reader is which files changed and how much, not
//! the hunk bodies - the bodies are exactly what the agent can re-read from the
//! repository. The reduction therefore walks the diff once, pairs every
//! `diff --git` header with the number of `@@` hunks that followed it, and emits
//! one line per file.
//!
//! Content with no `diff --git` header at all returns `None`: that is not a
//! diff, and summarising it as "0 files" would be a lie.

/// Reduce diff content: file-level summary with hunk counts.
pub fn ReduceDiff(Content:&str) -> Option<String> {
	let mut Files:Vec<(String, usize)> = Vec::new();
	let mut CurrentFile:Option<String> = None;
	let mut Hunks:usize = 0;

	for Line in Content.lines() {
		if Line.starts_with("diff --git ") {
			if let Some(f) = CurrentFile.take() {
				Files.push((f, Hunks));
			}
			CurrentFile = Some(Line.to_string());
			Hunks = 0;
		} else if Line.starts_with("@@ ") {
			Hunks += 1;
		}
	}
	if let Some(f) = CurrentFile {
		Files.push((f, Hunks));
	}

	if Files.is_empty() {
		return None;
	}

	let mut Parts = vec![format!("[diff:{} files]", Files.len())];
	for (Fname, Hcount) in &Files {
		Parts.push(format!("  {} ({} hunks)", Fname, Hcount));
	}
	Some(Parts.join("\n"))
}
