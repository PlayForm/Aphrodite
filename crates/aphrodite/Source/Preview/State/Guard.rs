//! Test-only serialisation for the process-wide cap.

/// Serialises tests across modules that mutate the process-global preview cap.
///
/// `cargo test` runs module test bodies concurrently and the cap is process-wide, so
/// the configuration-loader `ApplyPreviews` test and this module's cap tests must
/// not interleave. It is not a lock on preview building - it guards the mutation of
/// one atomic and nothing else.
///
/// A poisoned mutex is recovered rather than propagated: one failing test must not
/// cascade into unrelated failures, and the guarded state is a single integer that
/// cannot be left inconsistent.
#[cfg(test)]
pub(crate) fn Fn() -> std::sync::MutexGuard<'static, ()> {
	static G:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
	G.get_or_init(|| std::sync::Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}
