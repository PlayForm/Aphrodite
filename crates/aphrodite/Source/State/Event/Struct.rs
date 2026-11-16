//! The per-tool-call telemetry ring's own field.
//!
//! Split out of `AphroditeState` so the ring's readers and writers can take
//! `&mut Event::Struct` or `&Event::Struct`: phase detection, error-loop
//! breaking and the window statistics all scan the ring, and none of them needs
//! the marker catalog, the store or the configuration the session also carries.
//!
//! `Entry` is the item; this struct is the bounded collection.

use std::collections::VecDeque;

use super::Entry::Struct::Struct as Entry;

/// The bounded ring of per-call events (cap `Event::Const::Cap`, evict front).
///
/// `Default` is the seed: an empty ring.
#[derive(Clone, Default)]
pub struct Struct {
	/// Bounded ring of per-tool-call events (cap 200, evict front). Feeds phase
	/// detection, error-loop breaking, delta previews, checkpoints (P6-P11).
	pub Recent:VecDeque<Entry>,
}
