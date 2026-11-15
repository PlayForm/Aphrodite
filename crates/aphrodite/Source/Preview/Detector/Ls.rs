//! Directory-listing shape detector.
//!
//! A listing is recognised by `ls -l` mode strings or by a majority of bare
//! path-like tokens. This detector votes on those. It is not a filesystem check -
//! nothing is stat'ed, so the text is all that is judged.

use crate::Preview::{Input::Struct::Struct as Input, Line::Path};

/// True when the content looks like a directory listing.
///
/// `ls -l` mode strings, or a majority of bare path-like tokens (find or plain
/// `Ls`). It is not a path validator: a token that merely looks like a path
/// counts, which is why the majority gate exists.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let LsLong = Inp
		.NonEmpty
		.iter()
		.filter(|l| {
			let b = l.as_bytes();
			b.len() >= 10
				&& matches!(b[0], b'-' | b'd' | b'l' | b'c' | b'b' | b'p' | b's')
				&& b[1..10]
					.iter()
					.all(|&c| matches!(c, b'r' | b'w' | b'x' | b'-' | b's' | b't' | b'S' | b'T'))
		})
		.count();

	match LsLong >= 2 {
		true => {
			return true;
		},
		false => {},
	}

	let PathLines = Inp.NonEmpty.iter().filter(|l| Path::Fn(l)).count();

	PathLines >= 3 && PathLines * 2 >= Inp.NonEmpty.len()
}
