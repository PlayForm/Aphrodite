//! The registration as one value.

use super::super::Grammar::Enum::Enum as Grammar;

/// Everything a marker render needs from the configuration, published as one
/// value.
///
/// The two fields are read together on every marker, so they are stored
/// together: as three separate process-globals a render could observe a format
/// from one registration and a hint from the next, and a hot reload lands
/// exactly between them. One lock and one value make that mixture
/// unrepresentable.
///
/// `Hint` is `Option<String>` rather than the `String` + `AtomicBool` pair this
/// replaces, because the renderer cannot distinguish "the `[prompts]
/// ccr_marker_hint` knob is off" from "the knob is on and no hint string is
/// registered" - both emit nothing. The knob decides at registration time, which
/// is the only place the difference exists.
pub(crate) struct Struct {
	pub(crate) Grammar:Grammar,
	pub(crate) Hint:Option<String>,
}
