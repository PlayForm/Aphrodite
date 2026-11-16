//! The inline store's entry-count bound.

/// Entries the inline store may hold before the eviction pass drops the
/// least-recently-used one.
///
/// The LRU order is sized one larger than this so the cache never self-evicts
/// an entry the explicit pass would have accounted for - get that backwards and
/// the running byte total drifts from the map.
pub(crate) const Cap:usize = 500;
