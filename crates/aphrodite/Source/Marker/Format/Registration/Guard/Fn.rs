//! Serialising the tests that mutate the registration.

/// Hold this for the length of any test that registers a marker template.
///
/// `cargo test` runs test bodies concurrently and the registration is
/// process-wide, so `ConfigurationLoader`'s `ApplyCompression` tests and the
/// marker template tests must not interleave. One mutex, shared by every such
/// test, is the whole mechanism - it is compiled only under `cfg(test)`.
pub(crate) fn Fn() -> std::sync::MutexGuard<'static, ()> {
	static Serialised:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

	Serialised
		.get_or_init(|| std::sync::Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}
