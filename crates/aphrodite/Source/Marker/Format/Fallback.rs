//! The canonical marker grammar.
//!
//! This is the output every consumer of a marker actually depends on:
//! `<<<CCR:{hash}|{type}|{size}>>>` on the first line, the preview on the
//! second, then the optional hint and the optional `[meta:...]` line. It is
//! used whenever no `[templates.marker] format` is registered, which is the
//! shipped configuration.

use super::Bracket::IsSelfBracketedPreview;

/// Render the canonical marker grammar (fallback when `[templates.marker]
/// format` is unconfigured).
///
/// Doubling-bug fix (report 09 §5): `BuildPreview` already returns a
/// self-describing, fully bracketed preview like `[text:53L 1913B]` or
/// `[git:5M 2A | src/x.rs]`. Re-wrapping that in `[{center_str}:{preview}]`
/// produced the visible `[text:[text:53L 1913B]]` doubling. When the preview
/// is already a `[label:...]`-shaped string, emit it verbatim on the preview
/// line; only wrap bare previews (e.g. cache-mode raw excerpts) in the
/// `[{center_str}:...]` frame. A preview is thus produced exactly once.
///
/// The `Hint` text is appended to the preview line only when the
/// `CCRMarkerHint` knob is enabled (opt-in; empty = byte-identical to the
/// pre-knob output).
pub(super) fn RenderMarkerFallback(
	Preview:&str,
	CCRType:&str,
	meta:&str,
	Center:Option<&str>,
	hash:&str,
	Size:usize,
	Hint:&str,
) -> String {
	let CenterStr = Center.unwrap_or(CCRType);
	let MetaPart = if meta.is_empty() { String::new() } else { format!("\n[meta:{}]", meta) };

	let PreviewLine = if IsSelfBracketedPreview(Preview) {
		Preview.to_string()
	} else {
		format!("[{}:{}]", CenterStr, Preview)
	};

	format!("<<<CCR:{}|{}|{}>>>\n{}{}{}", hash, CCRType, Size, PreviewLine, Hint, MetaPart)
}
