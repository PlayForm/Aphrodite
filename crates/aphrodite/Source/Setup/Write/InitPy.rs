//! The Hermes `__init__.py` loader install.
//!
//! Writes the compiled-in loader (`PluginShim::Const`) into the plugin directory
//! on every run. It is code, not configuration, so it is never preserved: the
//! shim's FFI symbol list and registration logic have to match the dylib that was
//! just installed.

use std::fs;

use anyhow::Context;

use crate::Setup::{Run::Context::Struct as SetupContext, Write::Permissions::Fn as SecurePerms};
use super::PluginShim;

/// Write `__init__.py` into the plugin directory.
///
/// Always overwritten (03-F8): a re-run of `aphrodite setup` must refresh a
/// doctored or stale shim rather than keep it, because a shim older than the dylib
/// calls FFI symbols that no longer exist.
///
/// It is not a template and it writes no runtime data - the loader lives in the
/// hooks-only plugin directory and resolves binaries, configuration and state from
/// the runtime home. A shim left in place after an upgrade is the failure mode:
/// Hermes loads it, the first hook raises on a missing symbol, and the plugin
/// appears to be installed but silent.
pub(crate) fn Fn(Context:&SetupContext) -> anyhow::Result<()> {
	let path = Context.PluginDirectory.join("__init__.py");

	// Always overwritten (03-F8): the shim is code, not config - its FFI symbol
	// list and registration logic must match the freshly-installed dylib, so a
	// re-run of `aphrodite setup` must refresh a doctored/stale shim rather than
	// preserve it. It is the hooks-only loader in the Hermes plugin dir; the
	// runtime home carries binaries/config/state, never the loader.
	println!("writing __init__.py -> {}", path.display());
	fs::write(&path, PluginShim::Const).with_context(|| format!("write the plugin loader {}", path.display()))?;
	SecurePerms(&path, 0o644)?;
	Ok(())
}
