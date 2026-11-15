//! git status preview arm.
//!
//! A status listing is a tally by code plus the paths involved. This arm
//! renders both. It is not a repository query: it reads the text git printed, so
//! nothing is stat'ed and an empty listing previews as empty.

use crate::Preview::{Input::Struct::Struct as Input, Line::GitStatus};

/// Build a preview of `git status` output.
///
/// Promises a per-code tally (`M`, `A`, `D`, `??`) and the first few paths, so
/// the model learns how much is staged and which files are involved. It is not
/// a diff: the codes are reported as git wrote them and the content of a change
/// is never read.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	use std::collections::BTreeMap;
	let mut Tally:BTreeMap<char, usize> = BTreeMap::new();
	let mut Paths:Vec<String> = Vec::new();
	for Line in Inp.raw.lines() {
		if let Some(Code) = GitStatus::Fn(Line) {
			// Collapse the two columns to the most significant status char
			// (first non-space, non-`?` preferred, else the raw char).
			let Character = Code.chars().find(|c| *c != ' ').unwrap_or('?');
			*Tally.entry(Character).or_insert(0) += 1;
			if Paths.len() < 3 {
				let p = Line.get(3..).unwrap_or("").trim();
				if !p.is_empty() {
					// `R old -> new` renames: keep the new path.
					let p = p.rsplit(" -> ").next().unwrap_or(p);
					Paths.push(p.chars().take(40).collect());
				}
			}
		}
	}
	if Tally.is_empty() {
		return format!("[git:{}L]", Inp.Total);
	}
	// Emit tallies in a stable, readable order.
	let Order = ['M', 'A', 'D', 'R', 'C', 'U', 'T', '?', '!'];
	let mut Counts:Vec<String> = Vec::new();
	for c in Order {
		if let Some(n) = Tally.get(&c) {
			let Label = if c == '?' { "??".to_string() } else { c.to_string() };
			Counts.push(format!("{}{}", n, Label));
		}
	}
	let Total:usize = Tally.values().sum();
	let Shown = Paths.len();
	let More = if Total > Shown { format!(" +{} more", Total - Shown) } else { String::new() };
	if Paths.is_empty() {
		format!("[git:{}]", Counts.join(" "))
	} else {
		format!("[git:{} | {}{}]", Counts.join(" "), Paths.join(" "), More)
	}
}
