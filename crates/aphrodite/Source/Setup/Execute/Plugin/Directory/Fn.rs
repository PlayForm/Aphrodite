//! Taking the plugin directory, or leaving a catalog install alone.

use std::fs;

use anyhow::Context;

use crate::Setup::Plan;

/// Make the plugin directory ready for the loader, unless Hermes owns it.
///
/// The plugin dir holds ONLY the loader (`plugin.yaml` + `__init__.py`) that
/// registers hooks and tools; every runtime artifact - binaries, config, state -
/// lives in the runtime home.
///
/// A catalog install is report-only: when `<hermes-home>/plugins/aphrodite` is the
/// plugin's own install dir, placed there by `download.sh`/`download.ps1`, setup must
/// never write into it or delete it, and it must say so rather than silently
/// skipping. A stale symlink from an older install (plugin dir -> runtime home) is
/// removed first, so the loader files land as real files and never through the link
/// into the runtime home.
///
/// The catalog branch is a `then` block rather than an early return, so the decision
/// is one value read twice in the same order instead of a branch a reader has to
/// follow through the function.
pub(crate) fn Fn(Plan:&Plan::Struct::Struct) -> anyhow::Result<()> {
	let _ = Plan.CatalogInstall.then(|| {
		println!(
			"NOTE: {} is the plugin's own install dir under the Hermes home (catalog install) - report-only, left \
			 untouched",
			Plan.Context.PluginDirectory.display()
		);
		println!("  a catalog install must use `bash download.sh` / `pwsh download.ps1`, not `aphrodite setup`");
		println!("  `aphrodite setup` will not remove, create, or overwrite files there");
	});

	let Prepare = || -> anyhow::Result<()> {
		let _ = Plan.RemoveStaleSymlink.then(|| {
			println!("removing stale plugin symlink -> {}", Plan.Context.PluginDirectory.display());
		});

		let _ = Plan
			.RemoveStaleSymlink
			.then(|| {
				fs::remove_file(&Plan.Context.PluginDirectory).with_context(|| {
					format!("remove the stale plugin symlink {}", Plan.Context.PluginDirectory.display())
				})
			})
			.transpose()?;

		fs::create_dir_all(&Plan.Context.PluginDirectory)
			.with_context(|| format!("create the plugin directory {}", Plan.Context.PluginDirectory.display()))?;

		Ok(())
	};

	let _ = (!Plan.CatalogInstall).then(Prepare).transpose()?;

	Ok(())
}
