//! What the context engine decided.

use crate::Session::Engine::{Gate::Enum::Enum as Gate, Plan::Struct::Struct as Plan};

/// The engine's decision: refuse for a named reason, or archive a built plan.
///
/// A value rather than a sequence of early returns, because every refusal is a
/// true no-op and every archive is a fully prepared batch - so the caller can
/// tell "nothing happened" from "this is exactly what will happen" before either
/// becomes true.
pub enum Enum {
	/// Refused; nothing will be mutated.
	Skip(Gate),
	/// Archive this prepared plan.
	Archive(Plan),
}
