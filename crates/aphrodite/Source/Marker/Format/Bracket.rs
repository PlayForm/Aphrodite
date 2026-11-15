//! The "is this preview already self-describing?" test.
//!
//! The rule is deliberately stricter than "starts with `[`": a bare excerpt
//! that happens to begin with a bracket (a JSON array, a markdown link) must
//! still be wrapped, or the marker would claim a label it does not have. The
//! head must be `[word:` - a leading `[`, then an alphanumeric/underscore
//! label, then a colon - and the string must end with `]`.

/// True when `Preview` is already a self-describing `[label:...]` preview (as
/// produced by `BuildPreview`), so `render_marker` must not wrap it again.
/// Requires a leading `[`, a matching trailing `]`, and a `[word:` label head
/// (`\\[\\w+:`) so a bare excerpt that merely happens to start with `[` isn't
/// mistaken for a preview.
pub(super) fn IsSelfBracketedPreview(Preview:&str) -> bool {
	let p = Preview.trim();
	if !p.starts_with('[') || !p.ends_with(']') {
		return false;
	}
	let Inner = &p[1..];
	match Inner.find(':') {
		Some(Colon) => {
			let Label = &Inner[..Colon];
			!Label.is_empty() && Label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
		},
		None => false,
	}
}
