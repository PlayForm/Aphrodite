//! Markdown document preview arm.

use crate::preview::input::Input;
use crate::preview::line::md_heading::is_md_heading;

/// Markdown document preview: heading tally + first heading.
/// `[md:7L h1×1 h2×2 | # Release Notes]`.
pub(crate) fn build_markdown_preview(inp: &Input<'_>) -> String {
	let mut levels: Vec<usize> = Vec::new();
	let mut first_heading: Option<String> = None;
	for line in inp.raw.lines() {
		if is_md_heading(line) {
			let t = line.trim_start();
			let n = t.chars().take_while(|c| *c == '#').count();
			levels.push(n);
			if first_heading.is_none() {
				first_heading = Some(t.chars().take(48).collect());
			}
		}
	}
	if levels.is_empty() {
		return format!("[md:{}L]", inp.total);
	}
	let tally: Vec<String> = (1..=6)
		.filter_map(|l| {
			let c = levels.iter().filter(|&&x| x == l).count();
			if c > 0 { Some(format!("h{l}×{c}")) } else { None }
		})
		.collect();
	match first_heading {
		Some(h) => format!("[md:{}L {} | {}]", inp.total, tally.join(" "), h),
		None => format!("[md:{}L {}]", inp.total, tally.join(" ")),
	}
}
