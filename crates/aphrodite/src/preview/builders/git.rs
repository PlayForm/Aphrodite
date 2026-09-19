//! git status preview arm.

use crate::preview::input::Input;
use crate::preview::line::git_status::git_status_code;

/// git status preview: tally each two-char status code and list the first few
/// paths. `[git:5M 2A 1D 3?? | src/x.rs src/y.rs +6 more]`.
pub(crate) fn build_git_status_preview(inp: &Input<'_>) -> String {
	use std::collections::BTreeMap;
	let mut tally: BTreeMap<char, usize> = BTreeMap::new();
	let mut paths: Vec<String> = Vec::new();
	for line in inp.raw.lines() {
		if let Some(code) = git_status_code(line) {
			// Collapse the two columns to the most significant status char
			// (first non-space, non-`?` preferred, else the raw char).
			let ch = code.chars().find(|c| *c != ' ').unwrap_or('?');
			*tally.entry(ch).or_insert(0) += 1;
			if paths.len() < 3 {
				let p = line.get(3..).unwrap_or("").trim();
				if !p.is_empty() {
					// `R old -> new` renames: keep the new path.
					let p = p.rsplit(" -> ").next().unwrap_or(p);
					paths.push(p.chars().take(40).collect());
				}
			}
		}
	}
	if tally.is_empty() {
		return format!("[git:{}L]", inp.total);
	}
	// Emit tallies in a stable, readable order.
	let order = ['M', 'A', 'D', 'R', 'C', 'U', 'T', '?', '!'];
	let mut counts: Vec<String> = Vec::new();
	for c in order {
		if let Some(n) = tally.get(&c) {
			let label = if c == '?' { "??".to_string() } else { c.to_string() };
			counts.push(format!("{}{}", n, label));
		}
	}
	let total: usize = tally.values().sum();
	let shown = paths.len();
	let more = if total > shown { format!(" +{} more", total - shown) } else { String::new() };
	if paths.is_empty() {
		format!("[git:{}]", counts.join(" "))
	} else {
		format!("[git:{} | {}{}]", counts.join(" "), paths.join(" "), more)
	}
}
