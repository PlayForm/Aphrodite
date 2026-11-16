//! Rewrite a parsed chain into a command that marks each segment.

use crate::ChainSplit::Segment::{Const::Marker, Struct::Struct as Segment};

/// Build the rewritten command: each segment preceded by a marker echoed to
/// STDERR (not stdout). Markers on stderr never pollute stdout (files, pipes,
/// captured tool output stay clean); Hermes merges stderr into the terminal
/// result, so the transform hook still sees them and splits per segment.
///
/// Every segment but the first is prefixed, which is a property of the index
/// rather than a boolean the body has to test - the first segment is the one
/// whose output needs no boundary in front of it.
pub fn Fn(Segments:&[Segment]) -> String {
	let Parts:Vec<String> = Segments
		.iter()
		.enumerate()
		.map(|(Index, Seg)| {
			let Prefix = match Index {
				0 => String::new(),
				_ => format!("echo {} 1>&2 ; ", Marker),
			};

			format!("{Prefix}{}", Seg.Command)
		})
		.collect();

	Parts.join(" ; ")
}
