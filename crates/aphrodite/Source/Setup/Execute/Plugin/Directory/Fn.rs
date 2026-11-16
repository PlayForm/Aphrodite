//! Taking the plugin directory, or leaving a directory setup does not own alone.

use std::fs;

use anyhow::Context;

use crate::Setup::Plan::{self, Ownership};

/// Make the plugin directory ready for the loader, unless it is not setup's to touch.
///
/// The plugin dir holds ONLY the loader (`plugin.yaml` + `__init__.py`) that registers
/// hooks and tools; every runtime artifact - binaries, config, state - lives in the
/// runtime home.
///
/// One match over the ownership decision, one branch per case, and each branch says
/// what it did or why it did nothing:
///
/// - `Catalog` - Hermes' own install dir, placed by `download.sh`/`download.ps1`.
///   Report-only, and it says so rather than skipping silently.
/// - `Uninspectable` - the path is there and setup could not tell what it is. Also
///   report-only: the alternative is writing into a directory setup does not own.
/// - `DevStaleSymlink` - an older install's link into the runtime home. Removed first,
///   so the loader files land as real files and never through the link.
/// - `Dev` - nothing there. Created.
pub(crate) fn Fn(Plan:&Plan::Struct::Struct) -> anyhow::Result<()> {
	match &Plan.Ownership {
		Ownership::Enum::Enum::Catalog => {
			println!(
				"NOTE: {} is the plugin's own install dir under the Hermes home (catalog install) - report-only, left \
				 untouched",
				Plan.Context.PluginDirectory.display()
			);
			println!("  a catalog install must use `bash download.sh` / `pwsh download.ps1`, not `aphrodite setup`");
			println!("  `aphrodite setup` will not remove, create, or overwrite files there");

			Ok(())
		},
		Ownership::Enum::Enum::Uninspectable { Cause } => {
			println!(
				"NOTE: {} could not be inspected ({Cause}) - left untouched",
				Plan.Context.PluginDirectory.display()
			);
			println!("  setup does not write into a directory it cannot identify; fix the path and re-run");

			Ok(())
		},
		Ownership::Enum::Enum::DevStaleSymlink => {
			println!("removing stale plugin symlink -> {}", Plan.Context.PluginDirectory.display());

			fs::remove_file(&Plan.Context.PluginDirectory).with_context(|| {
				format!("remove the stale plugin symlink {}", Plan.Context.PluginDirectory.display())
			})?;

			fs::create_dir_all(&Plan.Context.PluginDirectory)
				.with_context(|| format!("create the plugin directory {}", Plan.Context.PluginDirectory.display()))?;

			Ok(())
		},
		Ownership::Enum::Enum::Dev => {
			fs::create_dir_all(&Plan.Context.PluginDirectory)
				.with_context(|| format!("create the plugin directory {}", Plan.Context.PluginDirectory.display()))?;

			Ok(())
		},
	}
}
