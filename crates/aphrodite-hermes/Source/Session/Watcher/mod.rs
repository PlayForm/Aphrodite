//! The opt-in config auto-reload watcher thread.
//!
//! Runtime-only: the watcher spawns from the non-test branch of
//! `Configuration::Template::Fn::config_template`, and the test branch builds
//! plain defaults instead (a test must never start a background thread that
//! watches the repo's live `aphrodite.toml`). The module is therefore compiled
//! out of the test harness rather than left dead there.

#[cfg(not(test))]
pub mod Fn;
