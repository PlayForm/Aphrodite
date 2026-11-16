//! The recent-marker catalog's bound.

/// Entries the marker ring may hold before the oldest is evicted.
///
/// The ring is front-evicting, so this is what keeps index 0 the oldest
/// survivor - the catalog renderer and the delta counters read the ring by
/// position, not by insertion order. Not observable to the LLM: it changes what
/// survives a turn, never what is rendered.
pub(crate) const Cap:usize = 200;
