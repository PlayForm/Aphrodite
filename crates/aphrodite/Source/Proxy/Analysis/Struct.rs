//! What one piece of content is, analysed once.
//!
//! The preview renderer and the metadata renderer used to each be handed the
//! raw content plus a separately detected name, so a marker could show a
//! preview arm and a metadata block that disagreed about what they were looking
//! at. `Analysis` is that shared answer: the content, the detected name the
//! marker grammar prints, and the line count both renderers count anyway.

/// One piece of content, analysed.
pub(crate) struct Struct<'a> {
	/// The content itself.
	pub(crate) Content:&'a str,
	/// The detected name - printed verbatim in the model-visible marker.
	pub(crate) Ct:&'static str,
	/// Line count, which every metadata producer and the marker's size use.
	pub(crate) Lines:usize,
}
