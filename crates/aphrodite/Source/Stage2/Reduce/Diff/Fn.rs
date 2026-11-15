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

	Content.lines().for_each(|Line| {
		match () {
			_ if Line.starts_with("diff --git ") => {
				match CurrentFile.take() {
					Some(f) => {
						Files.push((f, Hunks));
					},
					None => {},
				}
				CurrentFile = Some(Line.to_string());
				Hunks = 0;
			},
			_ if Line.starts_with("@@ ") => {
				Hunks += 1;
			},
			_ => {},
		}
	});
	match CurrentFile {
		Some(f) => {
			Files.push((f, Hunks));
		},
		None => {},
	}

	match Files.is_empty() {
		true => {
			return None;
		},
		false => {},
	}

	let mut Parts = vec![format!("[diff:{} files]", Files.len())];
	Files.iter().for_each(|(Fname, Hcount)| {
		Parts.push(format!("  {} ({} hunks)", Fname, Hcount));
	});
	Some(Parts.join("\n"))
}
