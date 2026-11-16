//! The template a test build starts from: the documented defaults.

use aphrodite::State::Struct::AphroditeState;

/// The plain-default template for test builds.
///
/// Test builds skip the TOML load on purpose: `Config::load()` searches
/// `./aphrodite.toml` first, and this crate's own tests run with the workspace
/// root (which HAS a real `aphrodite.toml`) as their working directory - without
/// this split, unit tests would non-hermetically pick up whatever the repo's live
/// config happens to contain instead of the documented defaults they assert
/// against.
///
/// It also starts no watcher: a test must never have a background thread
/// watching the repo's live config file.
pub(crate) fn Fn() -> AphroditeState { AphroditeState::default() }
