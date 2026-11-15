//! The `BINARY_VERSION` pin written into the runtime home.
//!
//! The loader resolves the version it was shipped against from the runtime home
//! for installed layouts, and falls back to its own source checkout in a dev tree
//! (which pins its own version). The pin therefore lives beside the binary, not in
//! the plugin directory: the plugin directory carries the loader and nothing else.

use std::fs;

use crate::Setup::{
	Run::{Context::Struct as SetupContext, Error::Enum as SetupError},
	Write::Permissions::Fn as SecurePerms,
};

/// Write `BINARY_VERSION` into the runtime home.
///
/// One line, the crate version, rewritten on every run so the pin always matches
/// the binary that was just installed.
///
/// It is not a compatibility record and nothing reads it to gate a load: a pin
/// that disagrees with the dylib is not detected here, it is reported by the
/// version handshake at plugin start. Writing it into the plugin directory instead
/// of the runtime home puts runtime data in a Hermes-owned directory and breaks
/// the catalog-install boundary.
pub(crate) fn Fn(Context:&SetupContext) -> Result<(), SetupError> {
	let path = Context.AphroditeDirectory.join("BINARY_VERSION");
	println!("writing BINARY_VERSION -> {}", path.display());
	fs::write(&path, format!("{}\n", env!("CARGO_PKG_VERSION")))?;
	SecurePerms(&path, 0o644)?;
	Ok(())
}
