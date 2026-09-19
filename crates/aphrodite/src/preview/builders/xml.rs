//! XML preview arm.

use crate::preview::input::Input;

/// XML preview: element count + root tag. `[xml:3 elements 4L | <root>]`.
pub(crate) fn build_xml_preview(inp:&Input<'_>) -> String {
	let elements = inp.raw.matches("</").count();
	let root = inp.raw.lines().map(|l| l.trim()).find(|l| l.starts_with('<')).map(|l| {
		let tag = l[1..]
			.split(|c:char| c.is_whitespace() || c == '>' || c == '/')
			.next()
			.unwrap_or("");
		format!("<{}>", tag)
	});
	match root {
		Some(r) => format!("[xml:{} elements {}L | {}]", elements, inp.total, r),
		None => format!("[xml:{} elements {}L]", elements, inp.total),
	}
}