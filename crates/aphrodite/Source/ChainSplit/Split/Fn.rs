//! Chain splitting: the scanner that turns one command string into segments.

use super::{Close::Fn::Fn as CloseSegment, Quote::Enum::Enum as Quoting};
use crate::ChainSplit::Segment::Struct::Struct as Segment;

/// Split a command string into segments on shell separators (`&&`, `;`,
/// newlines), respecting single/double quotes and trailing backslash
/// continuations. Returns `None` for constructs we must not rewrite.
///
/// The scanner matches on `(char, quoting state)` rather than on a character
/// with a guard per arm: the quoting state is the value that decides which
/// characters are significant, so it belongs in the pattern. Two booleans
/// (`in_single`, `in_double`) carry a third state - both set - that the shell
/// cannot reach, and every arm had to re-derive the same fact from them.
pub fn Fn(command:&str) -> Option<Vec<Segment>> {
	let mut Segments:Vec<Segment> = Vec::new();
	let mut Current = String::new();
	let mut Chars = command.chars().peekable();
	let mut Quoting = Quoting::None;
	let mut SegIndex = 0usize;

	while let Some(c) = Chars.next() {
		match (c, Quoting) {
			('\'', Quoting::None) => Quoting = Quoting::Single,
			('\'', Quoting::Single) => Quoting = Quoting::None,
			('"', Quoting::None) => Quoting = Quoting::Double,
			('"', Quoting::Double) => Quoting = Quoting::None,
			('\\', _) => {
				// Backslash continuation: swallow the next char (newline or
				// escaped char) so a multi-line command stays one segment.
				Chars.next().into_iter().for_each(|Escaped| {
					Current.push('\\');
					Current.push(Escaped);
				});

				continue;
			},
			('&', Quoting::None) => {
				// `&&` separator; a lone `&` (async) is left alone. The peek is the
				// decision, and its two answers are the two things to do.
				match Chars.next_if(|Next| *Next == '&') {
					Some(_) => SegIndex = CloseSegment(&mut Segments, &mut Current, SegIndex),
					None => Current.push(c),
				}

				continue;
			},
			(';', Quoting::None) | ('\n', Quoting::None) => {
				// A newline between commands is a separator; a trailing
				// backslash already merged the next line into `Current`.
				SegIndex = CloseSegment(&mut Segments, &mut Current, SegIndex);

				continue;
			},
			_ => {},
		}

		Current.push(c);
	}

	let _ = CloseSegment(&mut Segments, &mut Current, SegIndex);

	// Require at least 2 non-empty segments to be worth rewriting, and refuse
	// chains containing a heredoc marker or a `for`/`while` loop spanning
	// segments - rewriting those would corrupt control flow.
	let Segments:Vec<Segment> = Segments.into_iter().filter(|s| !s.command.trim().is_empty()).collect();
	let Unrewritable = Segments.iter().any(|s| {
		let Lower = s.command.to_lowercase();

		Lower.contains("<<") || Lower.contains("for ") || Lower.contains("while ")
	});

	(Segments.len() >= 2 && !Unrewritable).then_some(Segments)
}
