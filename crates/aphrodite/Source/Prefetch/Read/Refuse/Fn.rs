//! The prefetch refusal guard - the security boundary of this module.
//!
//! `Maintain/SECURITY.md` names this function as the guard, and the tests in
//! `Prefetch/Test/Refuse.rs` are its proof. It is deliberately a denylist
//! evaluated before any read: `.env` and `auth.json` by file name wherever they
//! live, anything under `~/.ssh`, anything under the Hermes home, and existing
//! non-file paths. Weakening any rule turns prefetch into a credential-absorption
//! path, because prefetched content is stored in CCR and retrievable by hash
//! afterwards.

use std::path::Path;

use crate::Prefetch::Read::{
	Guard::Fn::GuardAbsolute,
	Refuse::{
		Home::Fn::Fn as UnderHome,
		NonFile::Fn::Fn as NonFile,
		SSH::Fn::Fn as UnderSSH,
		Sensitive::Fn::Fn as Sensitive,
	},
};

/// Refuse a path that must never be prefetched - the core-crate equivalent of the
/// bridge crate's `read_path_guarded` (crates/aphrodite-hermes/Source/Tool/mod.rs),
/// same refusal intent, 1:1 semantics, implemented here so the core crate needs no
/// dependency on the bridge crate:
///
/// - `.env` / `auth.json` files, wherever they live (even inside the workspace -
///   prefetch must never absorb credentials),
/// - anything under `~/.ssh`,
/// - anything under the Hermes home (`~/.hermes`, or `$HERMES_HOME` when set -
///   resolved the same way the bridge does via `Home::Hermes::Fn::Fn`),
/// - existing non-file paths (directories etc.).
///
/// Returns the refusal reason, or `None` when the path is allowed through to the
/// size/read checks.
///
/// The rules are four alternatives in the guard's order, and the resolution work
/// they share sits inside the closure so a path refused by its own file name is
/// never canonicalised or stat'ed. `~`-expansion and canonicalisation are the most
/// expensive thing this guard does, and a `.env` in the cwd is refused without
/// them.
pub(in crate::Prefetch) fn RefuseReason(path:&Path) -> Option<String> {
	Sensitive(path).or_else(|| {
		let Canon = GuardAbsolute(path);
		let HermesHome = crate::Home::Hermes::Fn::Fn();
		let HermesHome = HermesHome.canonicalize().unwrap_or(HermesHome);

		UnderSSH(&Canon, path)
			.or_else(|| UnderHome(&Canon, &HermesHome, path))
			.or_else(|| NonFile(path))
	})
}
