//! `ApplyCompression` - the whole configuration, in section order.

use super::{
	super::Struct::Struct,
	Section::{ChainSplit, Debug, Directive, Engine, Flow, Prompt, Runtime, Template},
};
use crate::State::AphroditeState;

impl Struct {
	/// Load compression settings into an AphroditeState.
	///
	/// Every knob resolves through `Resolve` (override → env → TOML →
	/// default), and the appliers run in the order the configuration was
	/// originally applied. Two consequences are deliberate and easy to break:
	///
	/// - A found-but-broken `aphrodite.toml` does NOT fail here. It has
	///   already degraded to defaults in `Load`; this only copies the recorded
	///   reason onto `state.config_error` so `AphroditeStats` can say
	///   "defaults in effect (parse failed)" instead of looking unconfigured.
	/// - Missing `directives/` directories do not fail either - the builtin
	///   set is substituted, with a warning naming every probed path.
	pub fn ApplyCompression(&self, State:&mut AphroditeState) {
		// Parse-failure self-diagnosis: a found-but-broken aphrodite.toml
		// falls back to defaults - surface it in `AphroditeStats` so
		// "defaults in effect" is never indistinguishable from "config not
		// set" (the tracing warn alone is a no-op in the Hermes dylib).
		State.ConfigurationError = self.ParseFailure.clone();
		Engine::Apply(self, State);
		Flow::Apply(self, State);
		Runtime::Apply(self, State);
		Debug::Apply(self, State);
		ChainSplit::Apply(self, State);
		Directive::Apply(self, State);
		Template::Apply(self, State);
		Prompt::Apply(self, State);
	}
}
