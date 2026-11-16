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

	// Brackets, then a colon, then the label: each step is the `Option` the
	// previous one produced, and a failure anywhere is simply `false`.
	p.strip_prefix('[')
		.and_then(|Inner| Inner.strip_suffix(']').map(|_| Inner))
		.and_then(|Inner| Inner.find(':').map(|Colon| &Inner[..Colon]))
		.filter(|Label| !Label.is_empty() && Label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
		.is_some()
}
