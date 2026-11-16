//! Close the segment the scanner is building.

use crate::ChainSplit::Segment::Struct::Struct as Segment;

/// Push the segment under construction when it carries text, empty the buffer,
/// and answer the index the next segment takes.
///
/// The index advances whether or not anything was pushed, because a segment's
/// index is its position in the chain - an empty piece between two separators
/// still consumed a position, and renumbering the survivors would attribute a
/// segment's output to the wrong command.
pub(crate) fn Fn(Segments:&mut Vec<Segment>, Current:&mut String, Index:usize) -> usize {
	let Text = Current.trim().to_string();
	Current.clear();

	Segments.extend((!Text.is_empty()).then_some(Segment { Index, Command:Text }));

	Index + 1
}
