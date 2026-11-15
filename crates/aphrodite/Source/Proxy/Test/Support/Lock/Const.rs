//! The lock that serializes the `/reload` tests (they share a process-global
//! env var).

/// Serializes the `/reload` tests: they share the process-global
/// `APHRODITE_CONFIG_PATH` env var, so concurrent runs race on it (one
/// test's path can clobber the other's between `set_var` and the reload
/// call, making the reload read the wrong TOML).
pub(crate) static ReloadTestLock:std::sync::Mutex<()> = std::sync::Mutex::new(());
