//! The result of one config-reload pass, as the watcher sees it.

use crate::Session::Broadcast::Report;

/// One pass over the registry, with the config's own verdict folded in.
///
/// The distinction is the config's, not the sweep's: a file that does not parse
/// is never applied, so its pass has no threshold to report - it records the
/// failure on every session and keeps the previous values. Both variants carry
/// the sweep's reach, because "how many sessions heard this" is a question
/// about the pass either way.
pub(crate) enum Enum {
	/// The config parsed and was applied to every session the sweep could
	/// reach.
	Applied(Report::Struct::Struct),
	/// The config did not parse. Nothing was applied; every reachable session
	/// was told why (so `aphrodite_stats` can say "defaults in effect (parse
	/// failed)"), and its previous values are still in force.
	Failed(Report::Struct::Struct),
}
