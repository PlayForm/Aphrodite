//! Preview length cap (Issue #11 WS4): char-boundary-safe truncation.

/// Truncate a preview to `max_chars` chars on a char boundary. Preserves the
/// closing `]` (with a `…` marker) for self-bracketed previews so
/// `render_marker`/`parse_preview`/`chain_split` keep working after
/// truncation - a bare cut that drops `]` would re-trigger the
/// `[text:[text:...]]` double-wrap bug class (`marker.rs`). `0` = no cap.
pub(crate) fn apply_preview_cap(preview:&str, max_chars:usize) -> String {
	if max_chars == 0 || preview.chars().count() <= max_chars {
		return preview.to_string();
	}
	let keep_close = preview.trim_end().ends_with(']');
	if !keep_close || max_chars <= 1 {
		return preview.chars().take(max_chars).collect();
	}
	let mut out:String = preview.chars().take(max_chars - 2).collect();
	out.push('…');
	out.push(']');
	out
}