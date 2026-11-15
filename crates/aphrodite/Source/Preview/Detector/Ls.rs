//! Directory-listing shape detector.
//!
//! A listing is recognised by `ls -l` mode strings or by a majority of bare path-like tokens. This
//! detector votes on those. It is not a filesystem check - nothing is stat'ed, so the text is all that
//! is judged.

use crate::Preview::{Input::Struct::Struct as Input, Line::Path};

/// Longest mode string an `ls -l` line opens with.
const ModeLength:usize = 10;

/// True when the content looks like a directory listing.
///
/// `ls -l` mode strings, or a majority of bare path-like tokens (find or plain `Ls`). It is not a path
/// validator: a token that merely looks like a path counts, which is why the majority gate exists.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let Long = Inp
		.NonEmpty
		.iter()
		.filter(|Line| {
			let Bytes = Line.as_bytes();

			Bytes.len() >= ModeLength
				&& matches!(Bytes[0], b'-' | b'd' | b'l' | b'c' | b'b' | b'p' | b's')
				&& Bytes[1..ModeLength]
					.iter()
					.all(|Byte| matches!(Byte, b'r' | b'w' | b'x' | b'-' | b's' | b't' | b'S' | b'T'))
		})
		.count();

	Long >= 2 || Inp.Majority(Path::Fn, 3)
}
