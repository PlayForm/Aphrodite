//! Apply a freshly loaded config to every live session.

use crate::{Configuration::Reload::Fn::apply_config_reload, Session::Broadcast::Fn::Fn as for_each_session};

/// Apply `Configuration` to EVERY registered session, and report whether any of
/// them still wants auto-reload.
///
/// A broken file keeps the previous values and surfaces the failure on every
/// session instead: the loader's own warn has already fired, and `aphrodite_stats`
/// reports "defaults in effect (parse failed)" from this field. A parse failure
/// therefore always keeps watching - the next successful write is what clears it.
///
/// The load is the CALLER's, so this leaf is a function of a supplied
/// configuration and can be exercised without a file on disk.
pub(crate) fn Fn(Configuration:&aphrodite::ConfigurationLoader::Configuration::Struct::Struct) -> bool {
	match Configuration.ParseFailure.clone() {
		Some(Failure) => {
			for_each_session(|State| State.ConfigurationError = Some(Failure.clone()));

			true
		},
		None => for_each_session(|State| apply_config_reload(Configuration, State)),
	}
}
