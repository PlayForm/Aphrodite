//! `Template::Guard::Fn`: serialise tests over the registry.

use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

/// Serializes tests across modules that mutate the process-global template
/// registry (`cargo test` runs module test-bodies concurrently; the registry
/// is process-wide, so `config_loader`'s `ApplyTemplates` test and this
/// module's tests must not interleave).
///
/// Test-only by construction (`#[cfg(test)]` in `Template/mod.rs`): a release
/// build has no mutex here, so the render path pays nothing for it. The guard
/// is a plain `()` mutex, not a re-entrant one - a test that took it twice
/// would deadlock, which is the correct signal that it is testing itself.
pub(crate) fn Fn() -> MutexGuard<'static, ()> {
	static G:OnceLock<Mutex<()>> = OnceLock::new();
	G.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(PoisonError::into_inner)
}
