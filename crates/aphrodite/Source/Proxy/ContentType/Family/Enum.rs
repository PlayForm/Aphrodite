//! The content families this package makes decisions about.
//!
//! The detector's vocabulary is open-ended - the preview pipeline can name a
//! shape this proxy has no opinion on - so the detected name stays a
//! `&'static str` and travels into the model-visible marker verbatim.
//! `Family::Classify` reduces it to the set of families that actually change a
//! threshold or select a metadata producer, and the two consumers match this
//! enum instead of re-comparing the same string literals.
//!
//! `Other` is the honest catch-all: a named shape the proxy treats exactly as
//! it treats plain text. Keeping it distinct from `Text` means a reader can see
//! that the fallback is deliberate rather than a missing arm.

/// The families that select behaviour.
pub(crate) enum Enum {
	CodeRust,
	CodePython,
	CodeGo,
	CodeJS,
	Code,
	Error,
	Diff,
	Git,
	BuildOutput,
	Log,
	Linter,
	Json,
	ToolOutput,
	Text,
	/// A recognised shape with no family-specific handling here.
	Other,
}
