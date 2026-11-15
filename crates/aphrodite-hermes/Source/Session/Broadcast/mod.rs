//! Apply a closure to every registered session.
//!
//! Compiled in every build, not just production: the sweep is the apply half of
//! the config watcher (`Session::Watcher::Apply`), and a sweep that only exists
//! in the runtime build cannot be exercised by a test. The watcher's own thread
//! spawn stays runtime-only; this leaf does not.

pub mod Fn;
