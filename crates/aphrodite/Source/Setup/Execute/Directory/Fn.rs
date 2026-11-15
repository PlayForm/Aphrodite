//! Creating the runtime home and its `binaries/` directory.

use std::fs;

use anyhow::Context;

use crate::Setup::Run::Context::Struct as SetupContext;

/// Create the two directories every later step writes into.
///
/// Both are created before any artifact is copied, so a missing directory is
/// reported as a directory problem rather than as a copy failure three steps later.
/// The plugin directory is deliberately not created here - it belongs to the Hermes
/// home and its creation is conditional on the catalog-install boundary.
pub(crate) fn Fn(Context:&SetupContext) -> anyhow::Result<()> {
	fs::create_dir_all(&Context.BinariesDirectory)
		.with_context(|| format!("create the binaries directory {}", Context.BinariesDirectory.display()))?;

	fs::create_dir_all(&Context.AphroditeDirectory)
		.with_context(|| format!("create the runtime home {}", Context.AphroditeDirectory.display()))?;

	Ok(())
}
