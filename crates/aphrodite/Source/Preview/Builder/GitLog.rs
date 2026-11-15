//! git log preview arm.
//!
//! A log dump is unreadable in bulk, but its commit count and its first and last
//! subjects describe the range it covers. This arm renders those. It is not a
//! history reader: nothing is resolved, and only the text of the log is read.

use crate::Preview::Input::Struct::Struct as Input;

/// Build a preview of `git log` output.
///
/// Promises the commit count plus the first and last short hash and subject, so
/// the model knows how far the range reaches and what its endpoints say. It is
/// not a graph: parents, branches and merges are not reconstructed.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	// Collect `commit <hash>` entries and, if present, the following subject.
	let all:Vec<&str> = Inp.raw.lines().collect();
	let mut Commits:Vec<(String, String)> = Vec::new();
	for (i, Line) in all.iter().enumerate() {
		if let Some(Rest) = Line.strip_prefix("commit ") {
			let hash:String = Rest.trim().chars().take(7).collect();
			// Subject: first non-empty, non-header line after the commit line.
			let Subject = all[i + 1..]
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
			Commits.push((hash, Subject));
		}
	}
	if Commits.is_empty() {
		return format!("[gitlog:{}L]", Inp.Total);
	}
	let n = Commits.len();
	let First = &Commits[0];
	if n == 1 {
		return format!("[gitlog:1 commit | {} {}]", First.0, First.1);
	}
	let Last = &Commits[n - 1];
	format!("[gitlog:{} commits | {} {} → {} {}]", n, First.0, First.1, Last.0, Last.1)
}
