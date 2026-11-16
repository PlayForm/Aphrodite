//! What one piece of content is, analysed once.
//!
//! The preview renderer and the metadata renderer used to each be handed the
//! raw content plus a separately detected name, so a marker could show a
//! preview arm and a metadata block that disagreed about what they were looking
//! at. `Analysis` is that shared answer: the content, the detected name the
//! marker grammar prints, and the line count both renderers count anyway.
//!
//! The fields are private and there is exactly one constructor, which is what
//! makes a mixed value unrepresentable. The content is authoritative; the type
//! is a CLAIM about it (a caller may name `compress` for synthetic content); the
//! line count is a FACT about it and is always counted from the content it
//! travels with. When these fields were public, a renderer could be handed one
//! message's bytes beside another message's type and line count - which is
//! exactly what the marker path did when it re-pointed the content at the bytes
//! the store returned and kept the old counts.

/// One piece of content, analysed.
pub(crate) struct Struct<'a> {
	/// The content itself.
	Content:&'a str,
	/// The detected name - printed verbatim in the model-visible marker.
	Ct:&'static str,
	/// Line count, counted from `Content` at construction.
	Lines:usize,
}

impl<'a> Struct<'a> {
	/// Analyse `Content` under the type `Ct`.
	///
	/// `Ct` is the caller's claim and is taken as given - the detector fills it in
	/// for real content, and the synthetic producers name their own. The line
	/// count is not a claim: it is counted here, from this content, so the two
	/// halves can never describe different payloads.
	pub(crate) fn New(Content:&'a str, Ct:&'static str) -> Self {
		Struct { Content, Ct, Lines:Content.lines().count() }
	}

	/// The content itself.
	pub(crate) fn Content(&self) -> &'a str { self.Content }

	/// The detected name, as the marker prints it.
	pub(crate) fn Ct(&self) -> &'static str { self.Ct }

	/// The line count of `Content`.
	pub(crate) fn Lines(&self) -> usize { self.Lines }

	/// The content's byte length - what the marker reports as the stored size.
	pub(crate) fn Size(&self) -> usize { self.Content.len() }
}
