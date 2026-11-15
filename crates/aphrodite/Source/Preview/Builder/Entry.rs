//! Preview entry point.
//!
//! This is the only function the rest of the crate calls to render a preview: it
//! resolves the effective type, hands the work to the dispatch table, and applies
//! the configured length cap to whatever comes back. It is not a builder - it
//! renders nothing itself, and every path through it ends in the same cap.

use crate::Preview::{Input, State::Maximum::PreviewMaximumChars, Type};
use super::{Cap, Dispatch};

/// Build a compact, human-readable preview of compressed content.
///
/// The preview is shaped per content type - error and warning counts for build
/// output, added/removed lines for a diff, function and struct counts for source
/// code - so the model gets a useful summary instead of a generic byte count
/// wherever a richer signal exists.
///
/// A registered `[templates.preview.<family>.<ctype>]` format string renders
/// first; its result takes the same `PreviewMaximumChars` cap as every other path.
/// When nothing is configured for the type (`RenderPreview` returns `None`), the
/// per-arm dispatch runs unchanged.
///
/// It is not a compressor and it is not allowed to be optimistic: the preview is
/// the final, honest representation of the stored payload, so a wrong string here
/// is a defect even when the stored bytes are intact.
pub fn Fn(TypeStr:&str, Content:&str) -> String {
	let Rendered = crate::Template::Render::Fn::Fn(TypeStr, Content).filter(|R| !R.is_empty());

	match Rendered {
		Some(Rendered) => {
			return Cap::Fn(
				&Rendered,
				PreviewMaximumChars.load(std::sync::atomic::Ordering::Relaxed) as usize,
			);
		},
		None => {},
	}

	let Inp = Input::Struct::Struct::new(Content).unwrap_or_else(|| Input::Struct::Struct::Empty(Content));
	let Effective = Type::Fn::Fn(TypeStr, &Inp);
	let Preview = Dispatch::Fn(Effective.as_ref(), &Inp);
	Cap::Fn(
		&Preview,
		PreviewMaximumChars.load(std::sync::atomic::Ordering::Relaxed) as usize,
	)
}
