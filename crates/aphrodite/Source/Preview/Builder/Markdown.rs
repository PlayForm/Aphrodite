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

	Inp.raw.lines().for_each(|Line| {
		match MarkdownHeading::Fn(Line) {
			true => {
				let t = Line.trim_start();
				let n = t.chars().take_while(|c| *c == '#').count();

				Levels.push(n);

				match FirstHeading.is_none() {
					true => {
						FirstHeading = Some(t.chars().take(48).collect());
					},
					false => {},
				}
			},
			false => {},
		}
	});

	match Levels.is_empty() {
		true => {
			return format!("[md:{}L]", Inp.Total);
		},
		false => {},
	}

	let Tally:Vec<String> = (1..=6)
		.filter_map(|l| {
			let c = Levels.iter().filter(|&&x| x == l).count();

			match c > 0 {
				true => Some(format!("h{l}×{c}")),
				false => None,
			}
		})
		.collect();
	match FirstHeading {
		Some(h) => format!("[md:{}L {} | {}]", Inp.Total, Tally.join(" "), h),
		None => format!("[md:{}L {}]", Inp.Total, Tally.join(" ")),
	}
}
