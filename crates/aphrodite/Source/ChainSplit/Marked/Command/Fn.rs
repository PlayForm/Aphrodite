//! Rewrite a parsed chain into a command that marks each segment.

use crate::ChainSplit::{Const::SegMarker, Segment::Struct::Struct as Segment};

/// Build the rewritten command: each segment preceded by a marker echoed to
/// STDERR (not stdout). Markers on stderr never pollute stdout (files, pipes,
/// captured tool output stay clean); Hermes merges stderr into the terminal
/// result, so the transform hook still sees them and splits per segment.
pub fn Fn(Segments:&[Segment]) -> String {
	let mut Out = String::new();
	for (i, Seg) in Segments.iter().enumerate() {
		if i > 0 {
			Out.push_str(&format!("echo {} 1>&2 ; ", SegMarker));
		}
		Out.push_str(&Seg.command);
		Out.push_str(" ; ");
	}
	// Strip the trailing " ; ".
	if Out.ends_with(" ; ") {
		Out.truncate(Out.len() - 3);
	}
	Out
}
