//! grep/ripgrep preview arm.

use std::collections::BTreeSet;

use crate::preview::input::Input;
use crate::preview::line::grep::is_grep_line;

/// grep/ripgrep preview: hit count, distinct files, first location.
/// `[grep:38 hits in 9 files | src/x.rs:12 …]`.
pub(crate) fn build_grep_preview(inp: &Input<'_>) -> String {
	let mut hits = 0usize;
	let mut files: BTreeSet<String> = BTreeSet::new();
	let mut first: Option<String> = None;
	for line in inp.raw.lines() {
		if !is_grep_line(line) {
			continue;
		}
		hits += 1;
		let mut it = line.splitn(3, ':');
		let path = it.next().unwrap_or("");
		let lno = it.next().unwrap_or("");
		files.insert(path.to_string());
		if first.is_none() {
			first = Some(format!("{}:{}", path, lno).chars().take(48).collect());
		}
	}
	if hits == 0 {
		return format!("[grep:{}L]", inp.total);
	}
	match first {
		Some(loc) => format!("[grep:{} hits in {} files | {} …]", hits, files.len(), loc),
		None => format!("[grep:{} hits in {} files]", hits, files.len()),
	}
}
