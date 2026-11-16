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

	// A link is answered before anything is resolved, because a link is never a
	// catalog install: it is removed so the loader lands as real files, never through
	// it. The ownership of the real path is the fallback the link case skips.
	Metadata
		.file_type()
		.is_symlink()
		.then_some(Enum::Stale)
		.unwrap_or_else(|| Canonical(PluginDirectory, HermesHome))
}

/// Who owns a real directory: one that resolves inside the Hermes home is the plugin's
/// own install dir; one that resolves anywhere else is setup's to take.
///
/// Both canonicalisations have to succeed, so a failure on either is reported rather
/// than read as "not a catalog install".
fn Canonical(PluginDirectory:&Path, HermesHome:&Path) -> Enum {
	match (std::fs::canonicalize(PluginDirectory), std::fs::canonicalize(HermesHome)) {
		(Ok(Resolved), Ok(Home)) => Resolved.starts_with(&Home).then_some(Enum::Catalog).unwrap_or(Enum::Dev),
		(Err(e), _) => Enum::Uninspectable { Cause:format!("resolve {}: {e}", PluginDirectory.display()) },
		(_, Err(e)) => Enum::Uninspectable { Cause:format!("resolve the Hermes home {}: {e}", HermesHome.display()) },
	}
}
