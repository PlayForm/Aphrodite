//! Which events a watch is aimed at.

use notify::{Event, EventKind};

use crate::Boot::Listen::Watcher::Target;

/// Is this event a change to the watched file?
///
/// The filter is exact: the event's path must be the resolved target file. A
/// substring test (`contains("aphrodite.toml")`) also matched `aphrodite.toml.swp`
/// and `.aphrodite.toml.tmp`, and matched nothing when the loaded file had another
/// name. `Modify` and `Create` are both accepted because an atomic save - write a
/// temp file, rename it into place - reports the destination as created on some
/// backends and as modified on others.
pub fn Fn(Event:&Event, Target:&Target::Struct::Struct) -> bool {
	matches!(Event.kind, EventKind::Modify(_) | EventKind::Create(_))
		&& Event.paths.iter().any(|Path| Path == &Target.File)
}
