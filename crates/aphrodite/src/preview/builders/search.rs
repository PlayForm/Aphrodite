//! search preview arm.

use std::collections::BTreeSet;

use crate::preview::input::Input;
use crate::preview::line::grep::is_search_line;

/// Search preview: grep/ripgrep hit count, distinct files, first match location.
/// Matches the same `file:line:` shape as `content_detector::SEARCH_RESULT_PATTERN`
/// (Phase 4: structural `is_search_line`, no regex).
pub(crate) fn build_search_preview(inp:&Input<'_>) -> String {
	let mut hits = 0usize;
	let mut files:BTreeSet<String> = BTreeSet::new();
	let mut first:Option<String> = None;
	for line in inp.raw.lines() {
		if line.trim().is_empty() {
			continue;
		}
		if !is_search_line(line) {
			continue;
		}
		hits += 1;
		if let Some((path, rest)) = line.split_once(':') {
			files.insert(path.to_string());
			if first.is_none() {
				let lno = rest.split(':').next().unwrap_or("");
				first = Some(format!("{}:{}", path, lno).chars().take(48).collect());
			}
		}
	}
	if hits == 0 {
		return format!("[search:{}L]", inp.total);
	}
	match first {
		Some(loc) => format!("[search:{} hits in {} files | {} …]", hits, files.len(), loc),
		None => format!("[search:{} hits in {} files]", hits, files.len()),
	}
}