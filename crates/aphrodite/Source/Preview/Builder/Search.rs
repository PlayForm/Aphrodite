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
	for Line in Inp.raw.lines() {
		if Line.trim().is_empty() {
			continue;
		}
		if !Grep::Search::Fn(Line) {
			continue;
		}
		Hits += 1;
		if let Some((path, Rest)) = Line.split_once(':') {
			Files.insert(path.to_string());
			if First.is_none() {
				let Lno = Rest.split(':').next().unwrap_or("");
				First = Some(format!("{}:{}", path, Lno).chars().take(48).collect());
			}
		}
	}
	if Hits == 0 {
		return format!("[search:{}L]", Inp.Total);
	}
	match First {
		Some(Loc) => format!("[search:{} hits in {} files | {} ...]", Hits, Files.len(), Loc),
		None => format!("[search:{} hits in {} files]", Hits, Files.len()),
	}
}
