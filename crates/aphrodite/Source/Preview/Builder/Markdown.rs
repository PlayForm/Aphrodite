//! Markdown document preview arm.
//!
//! A document is described by its heading outline - how many at each level and
//! what the first one says. This arm renders that outline. It is not a renderer:
//! inline formatting, code fences and link targets are not interpreted.

use crate::Preview::{Input::Struct::Struct as Input, Line::MarkdownHeading};

/// Build a preview of a markdown document.
///
/// Promises a heading tally by level plus the first heading text. It is not a
/// table of contents: only headings are counted, so a document that is all prose
/// previews as zero headings rather than as unreadable.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let mut Levels:Vec<usize> = Vec::new();
	let mut FirstHeading:Option<String> = None;
	for Line in Inp.raw.lines() {
		if MarkdownHeading::Fn(Line) {
			let t = Line.trim_start();
			let n = t.chars().take_while(|c| *c == '#').count();
			Levels.push(n);
			if FirstHeading.is_none() {
				FirstHeading = Some(t.chars().take(48).collect());
			}
		}
	}
	if Levels.is_empty() {
		return format!("[md:{}L]", Inp.Total);
	}
	let Tally:Vec<String> = (1..=6)
		.filter_map(|l| {
			let c = Levels.iter().filter(|&&x| x == l).count();
			if c > 0 { Some(format!("h{l}×{c}")) } else { None }
		})
		.collect();
	match FirstHeading {
		Some(h) => format!("[md:{}L {} | {}]", Inp.Total, Tally.join(" "), h),
		None => format!("[md:{}L {}]", Inp.Total, Tally.join(" ")),
	}
}
