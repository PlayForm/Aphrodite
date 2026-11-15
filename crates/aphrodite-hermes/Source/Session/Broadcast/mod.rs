//! Apply a closure to every registered session.
//!
//! Runtime-only: the only caller is the config auto-reload watcher
//! (`Session::Watcher`), which is itself compiled out of the test harness. A
//! sweep that is never reachable in a test build is dead code there, so it
//! leaves with the watcher instead of failing `-D warnings`.

#[cfg(not(test))]
pub mod Fn;
