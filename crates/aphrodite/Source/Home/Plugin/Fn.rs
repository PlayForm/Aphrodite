//! `PluginDirectory`: the Hermes plugin loader directory.

use std::path::PathBuf;

use crate::Home::Hermes::Fn::Fn as HermesHome;

/// The Hermes plugin loader dir: `<hermes-home>/plugins/aphrodite` - the
/// hooks-only install location Hermes owns (plugin.yaml + `__init__.py`).
///
/// It is derived from the HERMES home, not the runtime home: this is where
/// Hermes looks for plugins, so a runtime home that moved must not drag the
/// loader with it. `aphrodite setup` writes here and Hermes reads here; the
/// two disagreeing is a plugin that silently stops loading.
pub fn Fn() -> PathBuf { HermesHome().join("plugins").join("aphrodite") }
