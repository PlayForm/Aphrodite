//! Single runtime-home resolution shared by the engine binary (`Aphrodite`)
//! and the Hermes bridge dylib (`aphrodite-hermes`).
//!
//! The runtime home is the user-data folder that holds `aphrodite.toml`,
//! `binaries/`, `directives/`, `ccr.db`, and the proxy logs. It must be
//! decided in exactly one place: historically the plugin shim (Python)
//! resolved it from `$HERMES_HOME` while the Rust half resolved it from
//! `$HOME`, so whenever `HERMES_HOME != $HOME/.hermes` (the shipped Docker
//! image, profile gateways) the two halves disagreed and the plugin disabled
//! itself (issue 40).
//!
//! This module resolves the same way the shim does (`$HERMES_HOME` ->
//! platform default, `hermes_constants.get_hermes_home()` precedence:
//! context override -> env var -> platform default), with `$APHRODITE_HOME`
//! staying as the explicit override above it; the shim additionally exports
//! its own decision into `$APHRODITE_HOME` (env setdefault at import), so in
//! a plugin process both halves converge on one directory by construction.
//!
//! The tree is grouped by WHAT the path names, not by who asks for it:
//!
//! - `Hermes/` - the Hermes home itself (the plugin loader's parent).
//! - `Runtime/` - the Aphrodite user-data folder and its precedence chain.
//! - `Plugin/`, `Configuration/`, `Directive/`, `Database/` - the four paths
//!   derived from the runtime home, one file each.
//! - `Environment/` - the env-var reader every resolver above goes through,
//!   plus the test mutex that serialises the tests which set those vars.
//! - `Tilde/` - the `~` expansion the Python shim applies.
//!
//! Nothing here decides behaviour: every function is a pure path computation
//! over the process environment, so a caller can always ask the same question
//! twice and get the same answer, and no tracked file ever carries an
//! absolute path. Adding a new path means adding a leaf, never a branch in an
//! existing one.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Home::Runtime::Fn::Fn`), so the file that defines an item is the only
//! place it is defined and there is exactly one way to spell it.

pub mod Configuration;
pub mod Database;
pub mod Directive;
pub mod Environment;
pub mod Hermes;
pub mod Plugin;
pub mod Runtime;
pub mod Tilde;

#[cfg(test)]
mod Test;
