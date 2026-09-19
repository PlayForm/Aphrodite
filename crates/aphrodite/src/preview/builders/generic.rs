//! generic preview arm (the honest fallback).

use crate::preview::input::Input;
use crate::preview::text::first_meaningful::first_meaningful_line;
use crate::preview::text::sample_long::sample_long_line;

/// Plain-text / unrecognized fallback: even when we can't classify the shape,
/// do better than a bare L/B count - show a content hint so the agent has
/// SOME signal about what the blob is.
///
/// ISSUE-11 residuals #1/#4 (RC-D): the hint used to be the FIRST non-empty
/// line raw - a pretty-printed JSON blob previewed as a lone `{` (MISLEADING,
/// the top battery offender) and a 10 KB single-line payload as a bare 60-char
/// head (SHALLOW). Now: skip structural noise lines (lone braces/brackets) to
/// the first MEANINGFUL line (the first key / statement), and sample
/// head+tail for very long lines.
pub(crate) fn build_generic_preview(type_str: &str, inp: &Input<'_>) -> String {
	let hint = first_meaningful_line(inp.raw)
		.map(|l| sample_long_line(&l))
		.filter(|s| !s.is_empty());
	match hint {
		Some(h) => format!("[{}:{}L {}B | {}]", type_str, inp.total, inp.bytes, h),
		None => format!("[{}:{}L {}B]", type_str, inp.total, inp.bytes),
	}
}
