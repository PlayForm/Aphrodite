//! Applying a whole document at startup - every section, directives included.

use crate::State::Struct::AphroditeState;
use super::{super::super::Struct::Struct, Const::Startup};

impl Struct {
	/// Load the whole document into an `AphroditeState` - the startup pass.
	///
	/// Every knob resolves through `Resolve` (override → env → TOML → default) and
	/// every section runs, because a session that has just been created owns
	/// nothing yet: its directive set is discovered here and nowhere else.
	///
	/// Two consequences are deliberate and easy to break:
	///
	/// - A found-but-broken `aphrodite.toml` does NOT fail here. It has already
	///   degraded to defaults in `Load`; this only copies the recorded reason onto
	///   `state.config_error` so `AphroditeStats` can say "defaults in effect
	///   (parse failed)" instead of looking unconfigured.
	/// - Missing `directives/` directories do not fail either - the builtin set is
	///   substituted, with a warning naming every probed path.
	///
	/// This is not the reload path: a live session already owns its directive set,
	/// and re-running this against one would overwrite it. That is what
	/// `ApplyReload` exists for.
	pub fn ApplyCompression(&self, State:&mut AphroditeState) {
		State.ConfigurationError = self.ParseFailure.clone();

		Startup.iter().for_each(|Apply| Apply(self, State));
	}
}
