//! Resolving and preparing one listener's CCR database path.

use anyhow::Context;
use aphrodite::Configuration::CLI::Struct::Struct as CLI;

/// Make `CLI.CCRDatabasePath` absolute and create its parent directory.
///
/// A relative `ccr_db_path` is resolved against the **binary's** directory, not the
/// process's current directory, so the database lands in the same place regardless of
/// where the proxy was launched from. An absolute path is left exactly as the operator
/// wrote it, and an empty value stays empty - it means "unset", and the store then uses
/// its own default. Resolving either of those again would be a second opinion about a
/// decision already made.
///
/// Both halves live here because they are one decision: a path is not resolved until the
/// directory that will hold it exists, and a caller that took the resolved path without
/// the directory would fail on the first store open with a message about the store
/// rather than about the path.
///
/// Failure to create the parent directory is an error, not a warning: a listener whose
/// store cannot be opened answers every request while silently dropping every marker,
/// which is worse than refusing to start.
pub(crate) fn Fn(CLI:&mut CLI) -> anyhow::Result<()> {
	let Some(Configured) = CLI.CCRDatabasePath.clone() else {
		return Ok(());
	};

	// The one predicate: a non-empty relative path is the only shape that needs
	// resolving. Empty is "unset" and absolute is already final, so neither reaches the
	// executable-directory lookup at all.
	let NeedsResolution = !Configured.as_os_str().is_empty() && !Configured.is_absolute();

	if NeedsResolution {
		let ExeDirectory = std::env::current_exe()
			.ok()
			.and_then(|ExePath| ExePath.parent().map(std::path::Path::to_path_buf));

		if let Some(ExeDirectory) = ExeDirectory {
			let Resolved = ExeDirectory.join(&Configured);

			tracing::info!(
				"resolved relative ccr_db_path from {} to {}",
				Configured.display(),
				Resolved.display()
			);

			CLI.CCRDatabasePath = Some(Resolved);
		}
	}

	// The parent of whatever path is now in force, when there is one. Named in the error
	// because "create the database directory" without a path leaves the operator to guess
	// which of the two paths (configured or resolved) was wrong.
	if let Some(Parent) = CLI.CCRDatabasePath.as_ref().and_then(|p| p.parent()) {
		std::fs::create_dir_all(Parent)
			.with_context(|| format!("create the database directory {}", Parent.display()))?;
	}

	Ok(())
}
