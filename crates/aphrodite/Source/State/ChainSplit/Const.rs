//! The consequence ledger's bound.

/// Chain-split events the ledger may hold before the oldest is evicted.
///
/// The ring is what lets the adaptation read a *window* of consequences rather
/// than all of them; each evicted event takes its segment hashes out of the
/// attribution map with it, so the map holds exactly the identities the ring can
/// still explain.
pub(crate) const Cap:usize = 16;
