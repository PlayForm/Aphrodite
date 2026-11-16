//! Classifying the plugin directory.

use std::path::Path;

use super::Enum::Enum;

/// Decide who owns `PluginDirectory`, without writing anything.
///
/// The order of the questions is the policy:
///
/// 1. **Nothing there** - a fresh install. Setup creates the directory.
/// 2. **Something there that cannot be stat'ed** - unknown. Report-only: a permission
///    error is not "no catalog install", and treating it as one is how setup writes
///    into a directory it could not identify.
/// 3. **A symlink** - an older install's link into the runtime home. It is removed so
///    the loader lands as real files, never through the link. A catalog install is a
///    real directory, so a link is never one.
/// 4. **A real directory that resolves inside the Hermes home** - the plugin's own
///    install dir, placed by `download.sh`/`download.ps1`. Hermes owns it.
/// 5. **A real directory that resolves elsewhere** - setup's to take.
///
/// Steps 4 and 5 need both canonicalisations to succeed: "inside the Hermes home" is a
/// claim about real paths, and comparing an unresolved path against a resolved one
/// would answer no for a correct catalog install behind a symlinked `$HOME`.
pub(crate) fn Fn(PluginDirectory:&Path, HermesHome:&Path) -> Enum {
	let Metadata = match std::fs::symlink_metadata(PluginDirectory) {
		Ok(Metadata) => Metadata,
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Enum::Dev,
		Err(e) => {
			return Enum::Uninspectable { Cause:format!("inspect {}: {e}", PluginDirectory.display()) };
		},
	};

	if Metadata.file_type().is_symlink() {
		return Enum::DevStaleSymlink;
	}

	match (std::fs::canonicalize(PluginDirectory), std::fs::canonicalize(HermesHome)) {
		(Ok(Resolved), Ok(Home)) => {
			if Resolved.starts_with(&Home) {
				Enum::Catalog
			} else {
				Enum::Dev
			}
		},
		(Err(e), _) => Enum::Uninspectable { Cause:format!("resolve {}: {e}", PluginDirectory.display()) },
		(_, Err(e)) => Enum::Uninspectable { Cause:format!("resolve the Hermes home {}: {e}", HermesHome.display()) },
	}
}
