//! Assemble the final `<<<CCR:hash|type|size>>>` marker text.
//!
//! The marker is the entire contract with the model: hash, type and size, in
//! that order, with the preview and metadata around it. Formatting drift here
//! makes every stored entry unretrievable, because the parser on the other side
//! matches this shape exactly.

/// Edit this function to change the layout the LLM sees when content
/// is compressed. Three-line format by default: preview, structure, marker.
pub(crate) fn Fn(Preview:&str, Ct:&str, Metadata:&str, Center:Option<&str>, hash:&str, Size:usize) -> String {
	let CenterSeg = Center.map(|c| format!(";center={c}")).unwrap_or_default();
	format!("{Preview}\n[{Ct}: {Metadata}{CenterSeg}]\n<<<CCR:{hash}|{Ct}|{Size}>>>")
}
