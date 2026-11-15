//! Refuse anything under the Hermes home.

use std::path::Path;

/// Refuse a path inside the Hermes home (`~/.hermes`, or `$HERMES_HOME`).
///
/// That tree holds the agent's own configuration, credentials, session database
/// and plugin state; prefetching any of it would put it in the CCR store where a
/// retrieval could surface it. The home is resolved the same way the bridge does
/// and compared canonical, so a symlinked home cannot dodge the check.
pub(crate) fn Fn(Canon:&Path, HermesHome:&Path, path:&Path) -> Option<String> {
	Canon
		.starts_with(HermesHome)
		.then(|| format!("refusing path under Hermes home ({}): {path:?}", HermesHome.display()))
}
