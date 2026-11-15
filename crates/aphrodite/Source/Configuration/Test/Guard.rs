//! The environment-variable serializer.

/// Serializes tests that touch process-global env vars
/// (`APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT`), since `cargo test`
/// runs this module's tests concurrently by default.
pub(super) fn EnvironmentGuard() -> std::sync::MutexGuard<'static, ()> {
	static G:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
	G.get_or_init(|| std::sync::Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}
