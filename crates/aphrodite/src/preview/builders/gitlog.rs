//! git log preview arm.

use crate::preview::input::Input;

/// git log preview: commit count + first->last short hash and subject.
/// `[gitlog:20 commits | abc123 fix(x): … → def456 …]`.
pub(crate) fn build_gitlog_preview(inp:&Input<'_>) -> String {
	// Collect `commit <hash>` entries and, if present, the following subject.
	let all:Vec<&str> = inp.raw.lines().collect();
	let mut commits:Vec<(String, String)> = Vec::new();
	for (i, line) in all.iter().enumerate() {
		if let Some(rest) = line.strip_prefix("commit ") {
			let hash:String = rest.trim().chars().take(7).collect();
			// Subject: first non-empty, non-header line after the commit line.
			let subject = all[i + 1..]
				.iter()
				.map(|l| l.trim())
				.find(|l| {
					!l.is_empty()
						&& !l.starts_with("Author:")
						&& !l.starts_with("Date:")
						&& !l.starts_with("Merge:")
						&& !l.starts_with("commit ")
				})
				.unwrap_or("")
				.chars()
				.take(32)
				.collect::<String>();
			commits.push((hash, subject));
		}
	}
	if commits.is_empty() {
		return format!("[gitlog:{}L]", inp.total);
	}
	let n = commits.len();
	let first = &commits[0];
	if n == 1 {
		return format!("[gitlog:1 commit | {} {}]", first.0, first.1);
	}
	let last = &commits[n - 1];
	format!("[gitlog:{} commits | {} {} → {} {}]", n, first.0, first.1, last.0, last.1)
}
