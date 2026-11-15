//! The prefetch refusal guard - the security boundary of this module.
//!
//! `Maintain/SECURITY.md` names this function as the guard, and the tests in
//! `Prefetch/Test/Refuse.rs` are its proof. It is deliberately a denylist
//! evaluated before any read: `.env` and `auth.json` by file name wherever
//! they live, anything under `~/.ssh`, anything under the Hermes home, and
//! existing non-file paths. Weakening any arm turns prefetch into a
//! credential-absorption path, because prefetched content is stored in CCR
//! and retrievable by hash afterwards.

use std::path::Path;

use crate::Prefetch::Read::Guard::Fn::GuardAbsolute;

/// Refuse a path that must never be prefetched - the core-crate equivalent
/// of the bridge crate's `read_path_guarded`
/// (crates/aphrodite-hermes/Source/Tool/mod.rs), same refusal intent, 1:1
/// semantics, implemented here so the core crate needs no dependency on the
/// bridge crate:
///
/// - `.env` / `auth.json` files, wherever they live (even inside the
///   workspace - prefetch must never absorb credentials),
/// - anything under `~/.ssh`,
/// - anything under the Hermes home (`~/.hermes`, or `$HERMES_HOME` when
///   set - resolved the same way the bridge does via `Home::Hermes::Fn::Fn`),
/// - existing non-file paths (directories etc.).
///
/// Returns the refusal reason, or `None` when the path is allowed through
/// to the size/read checks.
pub(in crate::Prefetch) fn RefuseReason(path:&Path) -> Option<String> {
	// File-name denylist first - checked on the raw path so a relative
	// `.env` in the cwd is caught without any resolution.
	if let Some(name) = path.file_name().and_then(|n| n.to_str())
		&& (name == ".env" || name == "auth.json")
	{
		return Some(format!("refusing sensitive file name: {path:?}"));
	}
	let Canon = GuardAbsolute(path);
	if let Some(Home) = dirs::home_dir()
		&& Canon.starts_with(Home.join(".ssh"))
	{
		return Some(format!("refusing path under ~/.ssh: {path:?}"));
	}
	let Hh = crate::Home::Hermes::Fn::Fn();
	let Hh = Hh.canonicalize().unwrap_or(Hh);
	if Canon.starts_with(&Hh) {
		return Some(format!("refusing path under Hermes home ({}): {path:?}", Hh.display()));
	}
	if path.exists() && !path.is_file() {
		return Some(format!("refusing non-file path: {path:?}"));
	}
	None
}
