//! Re-applying a document to a live session - every section it has authority over.

use crate::State::Struct::AphroditeState;
use super::{super::super::Struct::Struct, Const::Sequence};

impl Struct {
	/// Re-apply the document to an existing `AphroditeState`, without directives.
	///
	/// A live session owns three things the document cannot know about: its
	/// directive set (swapped and extended at runtime by `aphrodite_directive`), its
	/// CCR store and its telemetry. `Apply::Startup::Fn` would re-probe the directives
	/// directory and reseed `ActiveDirectives` from `[directives] active`, silently
	/// resetting the model's directive selection mid-session.
	///
	/// The old shape of this was snapshot-and-restore: take `Directives` and
	/// `ActiveDirectives` out, run the startup pass, put them back. That worked, but
	/// it meant the reload path *possessed* the authority to replace them and then
	/// undid it - one refactor away from losing the restore and one copy-paste away
	/// from restoring the wrong pair. Here the authority is absent instead: the
	/// sequence this runs simply has no `Directive` step, so there is nothing to
	/// undo.
	///
	/// `ConfigurationError` is still recorded, because a broken file must be visible
	/// to `AphroditeStats` on every path that applied it.
	pub fn Reload(&self, State:&mut AphroditeState) {
		State.ConfigurationError = self.ParseFailure.clone();

		Sequence.iter().for_each(|Apply| Apply(self, State));
	}
}
