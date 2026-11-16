//! Preview extraction from a rendered marker line.
//!
//! The preview is the text inside `[label:...]` on the line under a marker.
//! This is a display-side read, so a malformed line returns `None` rather than
//! a best guess: a wrong preview is a misleading preview.

/// Parse the preview field from a marker line.
///
/// External contract: `Line` is the line rendered by `Marker::Format`, i.e.
/// `[label:preview]`, and the answer is `preview` without its label or brackets.
/// `None` means the line is not preview-shaped at all (no `[`, no `:`, no `]`, or
/// the closing bracket comes before the colon) - an empty preview is `Some("")`.
pub fn Fn(Line:&str) -> Option<String> {
	let Start = Line.find('[')?;
	let Colon = Line[Start..].find(':')?;
	let End = Line.rfind(']')?;
	match End > Start + Colon {
		true => Some(Line[Start + Colon + 1..End].to_string()),
		false => None,
	}
}
