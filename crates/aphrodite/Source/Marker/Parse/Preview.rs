//! Preview extraction from a rendered marker line.
//!
//! The preview is the text inside `[label:...]` on the line under a marker.
//! This is a display-side read, so a malformed line returns `None` rather than
//! a best guess: a wrong preview is a misleading preview.

/// Parse the preview field from a marker line.
pub fn ParsePreview(MarkerLine:&str) -> Option<String> {
	let Start = MarkerLine.find('[')?;
	let Colon = MarkerLine[Start..].find(':')?;
	let End = MarkerLine.rfind(']')?;
	if End > Start + Colon {
		Some(MarkerLine[Start + Colon + 1..End].to_string())
	} else {
		None
	}
}
