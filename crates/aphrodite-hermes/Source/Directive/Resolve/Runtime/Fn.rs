//! The shared runtime-home resolution.

use std::path::PathBuf;

/// `<runtime home>/directives/` from `aphrodite::Home::Runtime`.
///
/// One decision shared with the engine binary and the plugin shim
/// (`$HERMES_HOME`/aphrodite, then `$HOME/.hermes/aphrodite`, then the platform
/// user home), so directives can never land in a second, shadow home when
/// `HERMES_HOME != $HOME/.hermes` (issue 40).
pub(crate) fn Fn() -> Option<PathBuf> { aphrodite::Home::Runtime::Opt::Fn::Fn().map(|Home| Home.join("directives")) }
