//! Chain splitting: the scanner that turns one command string into segments.

use crate::ChainSplit::Segment::Struct::Struct as Segment;

/// Split a command string into segments on shell separators (`&&`, `;`,
/// newlines), respecting single/double quotes and trailing backslash
/// continuations. Returns `None` for constructs we must not rewrite.
pub fn Fn(command:&str) -> Option<Vec<Segment>> {
	let mut Segments = Vec::new();
	let mut Current = String::new();
	let mut Chars = command.chars().peekable();
	let mut InSingle = false;
	let mut InDouble = false;
	let mut SegIndex = 0usize;

	while let Some(c) = Chars.next() {
		match c {
			'\'' if !InDouble => InSingle = !InSingle,
			'"' if !InSingle => InDouble = !InDouble,
			'\\' => {
				// Backslash continuation: swallow the next char (newline or
				// escaped char) so a multi-line command stays one segment.
				match Chars.next() {
					Some(n) => {
						Current.push('\\');
						Current.push(n);
					},
					None => (),
				}

				continue;
			},
			'&' if !InSingle && !InDouble => {
				// `&&` separator; a lone `&` (async) is left alone.
				match Chars.peek() == Some(&'&') {
					true => {
						Chars.next();
						PushSegment(&mut Segments, &mut Current, SegIndex);
						SegIndex += 1;
					},
					false => {
						Current.push(c);
					},
				}

				continue;
			},
			';' if !InSingle && !InDouble => {
				PushSegment(&mut Segments, &mut Current, SegIndex);
				SegIndex += 1;
				continue;
			},
			'\n' if !InSingle && !InDouble => {
				// Newline between commands is a separator; a trailing
				// backslash already merged the next line into `Current`.
				PushSegment(&mut Segments, &mut Current, SegIndex);
				SegIndex += 1;
				continue;
			},
			_ => {},
		}
		Current.push(c);
	}
	PushSegment(&mut Segments, &mut Current, SegIndex);

	// Require at least 2 non-empty segments to be worth rewriting.
	let Segments:Vec<Segment> = Segments.into_iter().filter(|s| !s.command.trim().is_empty()).collect();

	match Segments.len() < 2 {
		true => return None,
		false => (),
	}

	// Refuse chains containing a heredoc marker or a `for`/`while` loop
	// spanning segments - rewriting those would corrupt control flow.
	match Segments.iter().any(|s| {
		let Lower = s.command.to_lowercase();

		Lower.contains("<<") || Lower.contains("for ") || Lower.contains("while ")
	}) {
		true => None,
		false => Some(Segments),
	}
}

fn PushSegment(Segments:&mut Vec<Segment>, Current:&mut String, Index:usize) {
	match !Current.trim().is_empty() {
		true => {
			Segments.push(Segment { Index, command:Current.trim().to_string() });
		},
		false => (),
	}

	Current.clear();
}
