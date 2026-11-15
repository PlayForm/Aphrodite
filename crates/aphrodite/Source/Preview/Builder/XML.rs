//! XML preview arm.
//!
//! An XML document is described by its root tag and how many elements it holds. This arm
//! renders those. It is not an XML parser: no validation and no namespace resolution, so a
//! malformed document still previews as a tag count.

use crate::Preview::Input::Struct::Struct as Input;

/// Build a preview of XML content.
///
/// Promises the element count and the root tag. It is not a tree: depth and attribute
/// structure are not reported, and the element count is the number of closing tags, so a
/// self-closing-only document reports zero.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Elements = Inp.raw.matches("</").count();
	let Root = Inp
		.raw
		.lines()
		.map(|Line| Line.trim())
		.find(|Line| Line.starts_with('<'))
		.map(|Line| {
			let Tag = Line[1..]
				.split(|Character:char| Character.is_whitespace() || Character == '>' || Character == '/')
				.next()
				.unwrap_or("");

			format!("<{Tag}>")
		});

	match Root {
		Some(Root) => format!("[xml:{Elements} elements {}L | {Root}]", Inp.Total),
		None => format!("[xml:{Elements} elements {}L]", Inp.Total),
	}
}
