//! The replacement a compressed result earns.

/// What Hermes should substitute for a tool/terminal result.
///
/// The borrow is deliberate: `Expanded` and `Marker` point into the payload the
/// hook just built, and the value is rendered in the same synchronous interval -
/// nothing queues it. `Chain` is owned because joining N markers produces text
/// that exists nowhere else.
pub(crate) enum Enum<'a> {
	/// Per-segment markers, one per line - the fine-grained chain-split form.
	Chain(String),
	/// The auto-expanded text, resolved inline so no marker reaches the model.
	Expanded(&'a str),
	/// One CCR marker, optionally preceded by the dev-only debug line.
	Marker(&'a str),
	/// No replacement: leave the original output untouched.
	Passthrough,
}
