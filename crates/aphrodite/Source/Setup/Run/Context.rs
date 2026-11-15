//! The resolved paths and identity every setup step reads.
//!
//! Setup resolves two roots once - the Hermes home, where the plugin loader
//! lives, and the runtime home, where binaries, configuration and state live -
//! and threads them through every step instead of letting each step re-derive
//! them. Deriving them twice is how an install ends up with its loader in one
//! home and its dylib in another.

use std::path::PathBuf;

/// Paths and self-identity resolved at the start of a setup run.
///
/// `AphroditeDirectory` is the runtime home (`<hermes-home>/aphrodite` for a default
/// install, or `$APHRODITE_HOME`), `BinariesDirectory` is its `binaries/`
/// subdirectory, `PluginDirectory` is the hooks-only loader directory under the Hermes
/// home, `OwnPath` is the running binary and `OwnHash` its BLAKE3.
///
/// It is not configuration: nothing here is user-editable, and it is not
/// persisted - the struct is built per run from the environment and dropped when
/// setup returns. Treating these as two homes is the point; collapsing them
/// reintroduces the second `~/.hermes/aphrodite` that issue 40 fixed.
pub(crate) struct Struct {
	pub(crate) AphroditeDirectory:PathBuf,
	pub(crate) BinariesDirectory:PathBuf,
	pub(crate) PluginDirectory:PathBuf,
	pub(crate) OwnPath:PathBuf,
	pub(crate) OwnHash:String,
}
