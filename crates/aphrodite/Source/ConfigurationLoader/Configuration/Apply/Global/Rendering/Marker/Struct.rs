//! The marker grammar and its hint line, as one value.

/// `[templates.marker]` plus the `[prompts] ccr_marker_hint` switch.
///
/// Format and hint are published together because they are one registration: a
/// reader that saw one document's format against another's hint would render a
/// marker neither document describes.
pub struct Struct {
	/// `[templates.marker] format`; `None` = the canonical fallback grammar.
	pub Format:Option<String>,
	/// `[templates.marker] hint`; `None` = no hint line.
	pub Hint:Option<String>,
	/// `[prompts] ccr_marker_hint`; `false` = no hint line is rendered even when
	/// `Hint` is set.
	pub Enable:bool,
}
