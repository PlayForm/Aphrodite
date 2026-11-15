//! The platform event source: `notify` events in, `()` out, no config knowledge.
//!
//! The filter is applied here rather than in the controller so a directory with
//! unrelated churn (an editor's swap files, the CCR database) never reaches the
//! channel at all - the channel holds 16 entries and a busy directory would
//! otherwise fill it with events nobody wants.

use std::path::Path;

use notify::{Event, EventKind, RecursiveMode, Watcher};

use super::Struct::Struct;

/// Start watching `WatchPath`'s directory, non-recursively.
///
/// Returns `None` - after warning - when the platform watcher cannot be created
/// or cannot start. Hot reload is a convenience, and a proxy that refuses to
/// start because `notify` failed is a worse outcome than one that needs a
/// restart to pick up a new threshold.
///
/// The filter is `Modify` events whose path contains `aphrodite.toml`: a rename
/// into place (the shape an atomic save uses) arrives as `Modify`, while
/// unrelated files in the same directory are dropped. `try_send` is deliberate -
/// a full channel means a reload is already pending, and blocking the platform
/// watcher's own thread to queue a duplicate would be worse than dropping it.
pub(crate) fn Fn(WatchPath:&Path) -> Option<Struct> {
	let (Tx, Rx) = tokio::sync::mpsc::channel(16);

	let mut Watcher = notify::recommended_watcher(move |Res:Result<Event, notify::Error>| {
		let IsRelevant = |Event:&Event| {
			matches!(Event.kind, EventKind::Modify(_))
				&& Event.paths.iter().any(|p| p.to_string_lossy().contains("aphrodite.toml"))
		};

		// `Res::Err` is a watch error, not an event: there is nothing to reload
		// from it, and `notify` reports a dead watch through the same channel it
		// reports events on.
		Res.ok().into_iter().filter(IsRelevant).for_each(|_| {
			let _ = Tx.try_send(());
		});
	})
	.map_err(|e| {
		tracing::warn!("failed to create config watcher: {e}");
		e
	})
	.ok()?;

	let WatchDirectory = WatchPath.parent().unwrap_or_else(|| Path::new("."));

	Watcher
		.watch(WatchDirectory, RecursiveMode::NonRecursive)
		.map_err(|e| {
			tracing::warn!("failed to start config watcher: {e}");
			e
		})
		.ok()?;

	Some(Struct::new(Rx, Watcher))
}
