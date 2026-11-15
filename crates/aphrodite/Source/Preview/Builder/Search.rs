//! Search preview arm.
//!
//! A search result differs from a grep result only in what counts as a hit, so
//! this arm reports the same three facts: hits, distinct files, first location.
//! It is not a ranking and it does not read match text.

use std::collections::BTreeSet;

use crate::Preview::{Input::Struct::Struct as Input, Line::Grep};

/// Build a preview of search results.
///
/// Promises the hit count, the number of distinct files and the first match
/// location. It is not the grep arm: the path here need not look file-ish, so a
/// result whose path has no extension still counts - which is the whole
/// difference between the two shapes.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let mut Hits = 0usize;
	let mut Files:BTreeSet<String> = BTreeSet::new();
	let mut First:Option<String> = None;

	Inp.raw
		.lines()
		.filter(|Line| !Line.trim().is_empty() && Grep::Search::Fn(Line))
		.for_each(|Line| {
			Hits += 1;

			match Line.split_once(':') {
				Some((path, Rest)) => {
					Files.insert(path.to_string());

					match First.is_none() {
						true => {
							let Lno = Rest.split(':').next().unwrap_or("");

							First = Some(format!("{}:{}", path, Lno).chars().take(48).collect());
						},
						false => {},
					}
				},
				None => {},
			}
		});

	match Hits == 0 {
		true => {
			return format!("[search:{}L]", Inp.Total);
		},
		false => {},
	}

	match First {
		Some(Loc) => format!("[search:{} hits in {} files | {} ...]", Hits, Files.len(), Loc),
		None => format!("[search:{} hits in {} files]", Hits, Files.len()),
	}
}
