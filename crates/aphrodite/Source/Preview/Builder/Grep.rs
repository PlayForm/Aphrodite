//! grep/ripgrep preview arm.
//!
//! A search result is described by three numbers and one location: how many
//! hits, how many distinct files, and where the first one is. This arm renders
//! them. It is not a match extractor - the first hit is shown verbatim and no
//! match text is parsed.

use std::collections::BTreeSet;

use crate::Preview::{Input::Struct::Struct as Input, Line::Grep};

/// Build a preview of grep/ripgrep output.
///
/// Promises the hit count, the number of distinct files and the first location.
/// It is not a relevance ranking: hits are counted as printed, so a file
/// mentioned twice counts once in the file total and twice in the hit total.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let mut Hits = 0usize;
	let mut Files:BTreeSet<String> = BTreeSet::new();
	let mut First:Option<String> = None;

	Inp.raw.lines().filter(|Line| Grep::Hit::Fn(Line)).for_each(|Line| {
		Hits += 1;
		let mut It = Line.splitn(3, ':');
		let path = It.next().unwrap_or("");
		let Lno = It.next().unwrap_or("");

		Files.insert(path.to_string());

		match First.is_none() {
			true => {
				First = Some(format!("{}:{}", path, Lno).chars().take(48).collect());
			},
			false => {},
		}
	});

	match Hits == 0 {
		true => {
			return format!("[grep:{}L]", Inp.Total);
		},
		false => {},
	}

	match First {
		Some(Loc) => format!("[grep:{} hits in {} files | {} ...]", Hits, Files.len(), Loc),
		None => format!("[grep:{} hits in {} files]", Hits, Files.len()),
	}
}
