//! What a setup run is going to do.

use crate::Setup::{Plan::Ownership, Run::Context::Struct as SetupContext};
use super::Step;

/// An inspectable installation plan.
///
/// Building one performs no writes: it resolves the two homes, hashes the running
/// binary, classifies the plugin directory, and produces the ordered step list. A
/// caller can therefore read - or print - exactly what a run would touch before it
/// touches anything.
///
/// `Ownership` replaced a pair of booleans (`CatalogInstall` and
/// `RemoveStaleSymlink`). The pair could describe a catalog install that also had a
/// stale symlink to remove - a state that cannot exist - and it could not describe
/// "the path is there and I could not tell what it is", which is the case where the
/// wrong answer writes into a directory setup does not own.
///
/// It is not a transaction log and it does not make the run reversible. Rollback
/// would need a journal of what each step replaced and a backup policy for it, and
/// neither exists; a plan says what will happen, not how to undo it.
pub(crate) struct Struct {
	/// The two resolved homes plus this binary's identity.
	pub Context:SetupContext,
	/// Who owns the plugin directory, and therefore what setup may do with it.
	pub Ownership:Ownership::Enum::Enum,
	/// The ordered steps, already filtered by the ownership decision.
	pub Steps:Vec<Step::Enum::Enum>,
}
