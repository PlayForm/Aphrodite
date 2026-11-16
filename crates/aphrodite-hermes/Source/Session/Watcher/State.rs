//! The process's one running config watcher, if it has one.

use std::sync::{Mutex, OnceLock};

use crate::Session::Watcher::Handle;

/// The running watcher's handle, or `None` before a spawn and after a stop.
///
/// One slot, because there is one config file: two watcher threads on the same
/// `aphrodite.toml` would apply every change twice. A second spawn therefore
/// takes the previous handle out of this slot and stops it (flag, wake, join)
/// before parking the new one - the thread it replaces is joined, not leaked.
///
/// The slot is also what gives the thread an owner. A detached thread's only
/// owner is the process; a handle in a static can be taken by a host that wants
/// the thread gone before it unloads this image.
pub(crate) static RUNNING:OnceLock<Mutex<Option<Handle::Struct::Struct>>> = OnceLock::new();
