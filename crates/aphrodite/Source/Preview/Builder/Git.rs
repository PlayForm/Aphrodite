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

	Inp.raw.lines().for_each(|Line| {
		match GitStatus::Fn(Line) {
			Some(Code) => {
				// Collapse the two columns to the most significant status char
				// (first non-space, non-`?` preferred, else the raw char).
				let Character = Code.chars().find(|c| *c != ' ').unwrap_or('?');

				*Tally.entry(Character).or_insert(0) += 1;

				match Paths.len() < 3 {
					true => {
						let p = Line.get(3..).unwrap_or("").trim();

						match !p.is_empty() {
							true => {
								// `R old -> new` renames: keep the new path.
								let p = p.rsplit(" -> ").next().unwrap_or(p);

								Paths.push(p.chars().take(40).collect());
							},
							false => {},
						}
					},
					false => {},
				}
			},
			None => {},
		}
	});

	match Tally.is_empty() {
		true => {
			return format!("[git:{}L]", Inp.Total);
		},
		false => {},
	}

	// Emit tallies in a stable, readable order.
	let Order = ['M', 'A', 'D', 'R', 'C', 'U', 'T', '?', '!'];
	let mut Counts:Vec<String> = Vec::new();

	Order.into_iter().for_each(|c| {
		match Tally.get(&c) {
			Some(n) => {
				let Label = match c == '?' {
					true => "??".to_string(),
					false => c.to_string(),
				};

				Counts.push(format!("{}{}", n, Label));
			},
			None => {},
		}
	});

	let Total:usize = Tally.values().sum();
	let Shown = Paths.len();
	let More = match Total > Shown {
		true => format!(" +{} more", Total - Shown),
		false => String::new(),
	};

	match Paths.is_empty() {
		true => format!("[git:{}]", Counts.join(" ")),
		false => format!("[git:{} | {}{}]", Counts.join(" "), Paths.join(" "), More),
	}
}
