//! `note_root_session`: publish the root session id.

use crate::Debug::Root::Store;

/// Publish the root session id (Hermes bridge `record_session`).
///
/// An empty id is ignored rather than stored: the bridge calls this on every
/// session start including subagents that carry no root, and storing "" would
/// make `LogPath` answer `None` for a session that HAS a root - the log would
/// silently go nowhere.
///
/// Whether the value needs storing at all is `Store`'s decision; this leaf only
/// decides whether there is a value to store.
pub fn Fn(Root:&str) { (!Root.is_empty()).then_some(Root).into_iter().for_each(Store::Fn::Fn); }
