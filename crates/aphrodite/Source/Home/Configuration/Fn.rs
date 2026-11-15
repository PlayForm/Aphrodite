//! `ConfigurationPath`: the `aphrodite.toml` under the runtime home.

use std::path::PathBuf;

use crate::Home::Runtime::Fn::Fn as RuntimeHome;

/// `aphrodite.toml` under the runtime home.
///
/// The name is fixed: the file watcher, the `/reload` handler and `setup` all
/// key off this exact leaf, so a configurable filename would need all three
/// to learn about it.
pub fn Fn() -> PathBuf { RuntimeHome().join("aphrodite.toml") }
