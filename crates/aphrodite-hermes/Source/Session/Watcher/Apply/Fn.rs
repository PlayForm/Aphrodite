//! Apply a freshly loaded config to every live session.

use crate::{
	Configuration::Reload::Fn::apply_config_reload,
	Session::{Broadcast::Fn::Fn as for_each_session, Watcher::Outcome},
};

/// Apply `Configuration` to EVERY registered session and report what the pass
/// did.
///
/// A broken file keeps the previous values and surfaces the failure on every
/// session instead: the loader's own warn has already fired, and `aphrodite_stats`
/// reports "defaults in effect (parse failed)" from this field. A parse failure
/// therefore always keeps watching - the next successful write is what clears it
/// (that decision is `Watcher::Continue`'s, not this leaf's).
///
/// The load is the CALLER's, so this leaf is a function of a supplied
/// configuration and can be exercised without a file on disk.
pub(crate) fn Fn(Configuration:&aphrodite::ConfigurationLoader::Configuration::Struct::Struct) -> Outcome::Enum::Enum {
	match Configuration.ParseFailure.clone() {
		Some(Failure) => {
			Outcome::Enum::Enum::Failed(for_each_session(|State| State.ConfigurationError = Some(Failure.clone())))
		},
		None => {
			// The process-global rendering settings are published ONCE per reload,
			// not once per session: they are one document's opinion about the whole
			// process, so a pass that reached no session must still publish them. A
			// broken file publishes nothing - the previous values stay in force,
			// exactly as they do for the per-session state.
			Configuration.ApplyGlobals();

			Outcome::Enum::Enum::Applied(for_each_session(|State| apply_config_reload(Configuration, State)))
		},
	}
}
