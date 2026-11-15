//! What a setup run is going to do.

use crate::Setup::Run::Context::Struct as SetupContext;
use super::Step;

/// An inspectable installation plan.
///
/// Building one performs no writes: it resolves the two homes, hashes the running
/// binary, decides whether the plugin directory is Hermes-owned, and produces the
/// ordered step list. A caller can therefore read - or print - exactly what a run
/// would touch before it touches anything.
///
/// It is not a transaction log and it does not make the run reversible. Rollback
/// would need a journal of what each step replaced and a backup policy for it, and
/// neither exists; a plan says what will happen, not how to undo it.
pub(crate) struct Struct {
	/// The two resolved homes plus this binary's identity.
	pub Context:SetupContext,
	/// The plugin directory is the plugin's own install dir under the Hermes home,
	/// placed there by a catalog install - report-only, never written.
	pub CatalogInstall:bool,
	/// The plugin directory is a stale symlink from an older install and must be
	/// removed before the loader can land as real files.
	pub RemoveStaleSymlink:bool,
	/// The ordered steps, already filtered by the conditions above.
	pub Steps:Vec<Step::Enum::Enum>,
}
