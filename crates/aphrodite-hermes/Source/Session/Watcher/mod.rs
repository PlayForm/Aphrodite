//! The opt-in config auto-reload watcher thread.
//!
//! The THREAD is runtime-only: it spawns from the non-test branch of
//! `Configuration::Template::Fn::config_template`, and a test must never start a
//! background thread watching the repo's live `aphrodite.toml`. The two pieces it
//! drives are not: `Debounce/` collapses an event burst from a plain channel and
//! `Apply/` applies a supplied config to the registry, so both stay compiled in
//! the test build and both are exercised there.

pub mod Apply;
pub mod Debounce;

#[cfg(not(test))]
pub mod Fn;
