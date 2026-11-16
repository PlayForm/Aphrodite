//! The three things that can happen to a write, spelled out.

/// What became of a configuration write.
///
/// The two ignored cases are separate variants because they are separate
/// decisions with separate causes, even though both leave the session untouched
/// and both answer `{"status":"ok"}`. Collapsing them - which is what the
/// silent `Err(_) => ()` this replaces did - makes "the host sent a number the
/// field cannot hold" indistinguishable from "the host sent a key this build
/// does not have", and the next person to debug a knob that will not move has
/// nothing to read.
///
/// Neither ignored case is a failure: `AphroditeConfigurationSet` is a runtime
/// knob, not a validated form, and a host that mistypes a key must not have its
/// session fail. The cost of that leniency is that a caller cannot tell a
/// rejected write from an applied one by the response alone - it has to read
/// the value back with `AphroditeConfigurationGet`.
pub(crate) enum Enum {
	/// The key is settable and its value parsed: the session was mutated.
	Applied,
	/// The key is settable but the value did not parse for its field's type.
	Invalid,
	/// The key is not settable by this export.
	Unknown,
}
