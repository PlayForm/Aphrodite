//! The platform event source: `notify` events in, the watch target out, no config
//! knowledge.
//!
//! The filter is applied here rather than in the controller, so a directory with
//! unrelated churn (an editor's swap files, the CCR database, the runtime home's own
//! state) never reaches the channel at all.

use notify::{Event, RecursiveMode, Watcher};

use super::Struct::Struct;
use crate::Boot::Listen::Watcher::{Select, Target};

/// Start watching the target's directory, non-recursively.
///
/// Returns the reason it could not start rather than `None`: hot reload is a
/// convenience, and a proxy that refuses to start because `notify` failed would be a
/// worse outcome than one that needs a restart - but a caller still has to be able to
/// say *why* hot reload is off, and `None` cannot.
///
/// The filter is exact: the event's path must be the resolved target file. A
/// substring test (`contains("aphrodite.toml")`) also matched `aphrodite.toml.swp`
/// and `.aphrodite.toml.tmp`, and matched nothing when the loaded file had another
/// name. `Modify` and `Create` are both accepted because an atomic save - write a
/// temp file, rename it into place - reports the destination as created on some
/// backends and as modified on others.
///
/// The channel holds ONE entry and the send is `try_send`: a full channel already
/// means "a reload is pending", so blocking the platform watcher's own thread to
/// queue a duplicate would be strictly worse than dropping it.
pub(crate) fn Fn(Target:&Target::Struct::Struct) -> Result<Struct, String> {
	let (Tx, Rx) = tokio::sync::mpsc::channel(1);

	// Dereferenced before cloning: `Target` is a `&Struct`, and `Target.clone()` on a
	// reference clones the REFERENCE, which cannot cross into a `'static` closure.
	let Watched = (*Target).clone();

	let mut Watcher = notify::recommended_watcher(move |Res:Result<Event, notify::Error>| {
		// A watch error is not an event: there is nothing to reload from it, and
		// `notify` reports a dead watch through the same channel it reports events on.
		Res.ok()
			.filter(|Event| Select::Fn::Fn(Event, &Watched))
			.map(|_| Tx.try_send(Watched.clone()))
			.into_iter()
			.for_each(drop);
	})
	.map_err(|e| format!("failed to create the config watcher: {e}"))?;

	Watcher
		.watch(&Target.Directory, RecursiveMode::NonRecursive)
		.map_err(|e| format!("failed to watch {}: {e}", Target.Directory.display()))?;

	Ok(Struct::new(Rx, Watcher))
}
