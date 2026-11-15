//! The `(offset, limit)` compatibility entry point.

use super::{Mode, Outcome, Render, Slice};

/// Slice `Content` to the requested line window, as the wire pair spells it.
///
/// This is the whole of the compatibility surface for the old signature: it
/// parses the pair into a [`Mode::Enum`], slices, and renders. Every decision
/// lives in those leaves, so there is no second copy of the window arithmetic
/// here and a caller that wants the typed form can take
/// `Page::Mode::Parse::Fn::Fn` + `Page::Slice::Fn::Fn` directly (the handler
/// does exactly that, because it needs the mode again for the live-container
/// progression).
///
/// `limit: 0` keeps its meaning through
/// [`super::Mode::Parse::Fn::Fn`]: no offset and no limit is the exact,
/// content-addressed document; a non-zero offset with no limit is the tail of the
/// document, annotated and truncated, exactly as before.
pub fn Fn(Content:&str, Offset:usize, Limit:usize) -> anyhow::Result<(String, bool)> {
	let Mode = Mode::Parse::Fn::Fn(Offset, Limit);
	let Outcome:Outcome::Enum::Enum = Slice::Fn::Fn(Content, &Mode)?;

	Ok((Render::Text::Fn::Fn(Content, &Outcome), Render::Truncated::Fn::Fn(&Outcome)))
}
