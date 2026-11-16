//! The settable keys, each carrying a value already in its field's type.

/// A configuration write that parsed, ready to be applied under the lock.
///
/// Each variant names one settable key and carries the value the session field
/// takes, so `Apply` cannot be handed a string that still needs parsing
/// and the parse cannot happen while the session lock is held. The integer
/// widths are the fields' own (`engine_threshold_pct` is `u64`,
/// `tool_threshold` is `usize`), which is why the two are separate variants
/// rather than one integer.
///
/// `api_url` is deliberately absent: it is readable through
/// `AphroditeConfigurationGet` and not settable, so a write to it is an unknown
/// key rather than a second spelling of a known one.
pub(crate) enum Enum {
	/// `model`; any string, including the empty one.
	Model(String),
	/// `engine_threshold_pct`; the session field is `u64`.
	Engine(u64),
	/// `tool_threshold`; the session field is `usize`.
	Tool(usize),
	/// `context_engine_enabled`; anything but `"true"` or `"1"` reads as `false`.
	Context(bool),
}
