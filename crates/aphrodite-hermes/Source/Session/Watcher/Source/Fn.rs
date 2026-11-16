//! Build the file notifier and start watching the config search directories.

use std::{path::PathBuf, sync::mpsc::Sender};

use notify::Watcher;

use crate::Session::Watcher::Select;

/// Build the notifier, wire its filter, and watch every directory in `Paths`.
///
/// Returns the notifier, which the CALLER must keep alive: dropping it stops
/// delivery, so the loop binds it for its whole lifetime rather than letting it
/// fall out of scope here.
///
/// Two failure classes, treated differently on purpose:
///
///   - the notifier itself cannot be built - `Err`, which is the spawner's
///     readiness failure and the reason `auto_reload = true` would otherwise
///     have done nothing;
///   - one directory cannot be watched - a warning, not a failure, because the
///     runtime home legitimately may not exist yet and the remaining
///     directories (the working directory, where `aphrodite.toml` usually is)
///     still deliver events.
///
/// The filter is the same one the engine's watcher uses: a `Modify` event whose
/// path mentions `aphrodite.toml`. Anything else on those directories - the
/// runtime home holds flags, logs and the hot-reload drop directory - is noise
/// the loop must not wake for.
pub(crate) fn Fn(Wake:Sender<()>, Paths:&[PathBuf]) -> Result<notify::RecommendedWatcher, String> {
	let mut Notifier = notify::recommended_watcher(move |Result:Result<notify::Event, notify::Error>| {
		Result
			.ok()
			.filter(Select::Fn::Fn)
			.map(|_| Wake.send(()))
			.into_iter()
			.for_each(drop);
	})
	.map_err(|e| format!("config auto-reload watcher failed to start: {e}"))?;

	Paths.iter().for_each(|Directory| {
		match Notifier.watch(Directory, notify::RecursiveMode::NonRecursive) {
			Ok(()) => {},
			Err(e) => {
				crate::Debug::Warn::Fn::Fn(&format!("config auto-reload: cannot watch {}: {e}", Directory.display()))
			},
		}
	});

	Ok(Notifier)
}
