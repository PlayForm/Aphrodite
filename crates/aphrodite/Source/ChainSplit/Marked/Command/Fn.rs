//! Rewrite a parsed chain into a command that marks each segment.

use crate::ChainSplit::{Const::SegMarker, Segment::Struct::Struct as Segment};

/// Build the rewritten command: each segment preceded by a marker echoed to
/// STDERR (not stdout). Markers on stderr never pollute stdout (files, pipes,
/// captured tool output stay clean); Hermes merges stderr into the terminal
/// result, so the transform hook still sees them and splits per segment.
pub fn Fn(Segments:&[Segment]) -> String {
	let mut Out = String::new();

	Segments.iter().enumerate().for_each(|(i, Seg)| {
		match i > 0 {
			true => {
				Out.push_str(&format!("echo {} 1>&2 ; ", SegMarker));
			},
			false => (),
		}

		Out.push_str(&Seg.command);
		Out.push_str(" ; ");
	});

	// Strip the trailing " ; ".
	match Out.ends_with(" ; ") {
		true => {
			Out.truncate(Out.len() - 3);
		},
		false => (),
	}

	Out
}
