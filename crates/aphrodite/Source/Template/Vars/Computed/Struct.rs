//! A variable derived from another variable.

/// The variables that are computed from the rest of the set.
///
/// One field, and it is a field rather than an inline expression because the
/// derivation has an order: `cmx` is the command excerpt, so it can only be computed
/// after `cmd` is known, and saying so in a type is what keeps the pass last.
pub(crate) struct Struct {
	/// The command, cut to 40 characters - `"?"` when the content carried none.
	pub(crate) Cmx:String,
}
