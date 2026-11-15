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
				if let Some(n) = Chars.next() {
					Current.push('\\');
					Current.push(n);
				}
				continue;
			},
			'&' if !InSingle && !InDouble => {
				// `&&` separator; a lone `&` (async) is left alone.
				if Chars.peek() == Some(&'&') {
					Chars.next();
					PushSegment(&mut Segments, &mut Current, SegIndex);
					SegIndex += 1;
					continue;
				}
				Current.push(c);
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
	if Segments.len() < 2 {
		return None;
	}
	// Refuse chains containing a heredoc marker or a `for`/`while` loop
	// spanning segments - rewriting those would corrupt control flow.
	for s in &Segments {
		let Lower = s.command.to_lowercase();
		if Lower.contains("<<") || Lower.contains("for ") || Lower.contains("while ") {
			return None;
		}
	}
	Some(Segments)
}

fn PushSegment(Segments:&mut Vec<Segment>, Current:&mut String, Index:usize) {
	if !Current.trim().is_empty() {
		Segments.push(Segment { Index, command:Current.trim().to_string() });
	}
	Current.clear();
}
