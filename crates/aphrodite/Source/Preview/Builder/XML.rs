//! XML preview arm.
//!
//! An XML document is described by its root tag and how many elements it holds.
//! This arm renders those. It is not an XML parser: no validation and no
//! namespace resolution, so a malformed document still previews as a tag count.

use crate::Preview::Input::Struct::Struct as Input;

/// Build a preview of XML content.
///
/// Promises the element count and the root tag. It is not a tree: depth and
/// attribute structure are not reported, and the element count is the number of
/// closing tags, so a self-closing-only document reports zero.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Elements = Inp.raw.matches("</").count();
	let Root = Inp.raw.lines().map(|l| l.trim()).find(|l| l.starts_with('<')).map(|l| {
		let Tag = l[1..]
			.split(|c:char| c.is_whitespace() || c == '>' || c == '/')
			.next()
			.unwrap_or("");
		format!("<{}>", Tag)
	});
	match Root {
		Some(r) => format!("[xml:{} elements {}L | {}]", Elements, Inp.Total, r),
		None => format!("[xml:{} elements {}L]", Elements, Inp.Total),
	}
}
