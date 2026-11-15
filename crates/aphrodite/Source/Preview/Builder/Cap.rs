//! Preview length cap.
//!
//! `PreviewMaximumChars` is a hard budget, and every arm passes through this one
//! truncation. Doing it in one place is what makes the cap a real guarantee
//! rather than a per-arm promise. It is not a byte truncation: the cut lands on
//! a character boundary, because a cut inside a multi-byte character would
//! panic.

/// Truncate a preview to `MaximumChars` characters on a character boundary.
///
/// Preserves the closing `]` (with a `...` marker) of a self-bracketed preview,
/// so `render_marker`/`ParsePreview`/`chain_split` keep working after
/// truncation. A bare cut that drops `]` re-triggers the `[text:[text:...]]`
/// double-wrap bug class, which is why the bracket is kept rather than the last
/// character. `0` means no cap - not "truncate to nothing".
pub(crate) fn Fn(Preview:&str, MaximumChars:usize) -> String {
	if MaximumChars == 0 || Preview.chars().count() <= MaximumChars {
		return Preview.to_string();
	}
	let KeepClose = Preview.trim_end().ends_with(']');
	if !KeepClose || MaximumChars <= 3 {
		return Preview.chars().take(MaximumChars).collect();
	}
	let mut Out:String = Preview.chars().take(MaximumChars - 4).collect();
	Out.push_str("...");
	Out.push(']');
	Out
}
