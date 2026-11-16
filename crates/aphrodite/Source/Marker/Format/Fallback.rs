//! The canonical marker grammar.
//!
//! This is the output every consumer of a marker actually depends on:
//! `<<<CCR:{hash}|{type}|{size}>>>` on the first line, the preview on the
//! second, then the optional hint and the optional `[meta:...]` line. It is
//! used whenever no `[templates.marker] format` is registered, which is the
//! shipped configuration.

use super::Bracket::Fn as Bracketed;

/// Render the canonical marker grammar (fallback when `[templates.marker]
/// format` is unconfigured).
///
/// External contract: the rendered bytes are the wire format the model reads and
/// `Marker::Parse` reads back - `<<<CCR:{hash}|{type}|{size}>>>`, a newline, the
/// preview line, then `Hint` and `[meta:...]` when they are non-empty. An empty
/// `Meta` drops the `[meta:...]` line entirely; an empty `Hint` appends nothing.
///
/// Doubling-bug fix (report 09 §5): `Preview::Builder::Entry::Fn` already returns
/// a self-describing, fully bracketed preview like `[text:53L 1913B]` or
/// `[git:5M 2A | src/x.rs]`. Re-wrapping that in `[{center_str}:{preview}]`
/// produced the visible `[text:[text:53L 1913B]]` doubling. When the preview
/// is already a `[label:...]`-shaped string, emit it verbatim on the preview
/// line; only wrap bare previews (e.g. cache-mode raw excerpts) in the
/// `[{center_str}:...]` frame. A preview is thus produced exactly once.
///
/// The `Hint` text is appended to the preview line only when the
/// `[prompts] ccr_marker_hint` knob is enabled (opt-in; empty = byte-identical
/// to the pre-knob output).
pub(super) fn Fn(Preview:&str, Type:&str, Meta:&str, Center:Option<&str>, Hash:&str, Size:usize, Hint:&str) -> String {
	let Frame = Center.unwrap_or(Type);
	let Trailer = match Meta.is_empty() {
		true => String::new(),
		false => format!("\n[meta:{}]", Meta),
	};

	let Line = match Bracketed(Preview) {
		true => Preview.to_string(),
		false => format!("[{}:{}]", Frame, Preview),
	};

	format!("<<<CCR:{}|{}|{}>>>\n{}{}{}", Hash, Type, Size, Line, Hint, Trailer)
}
