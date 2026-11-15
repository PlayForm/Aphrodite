//! The conversation-index cap.

/// Cap on `ConvIndex` entries - the eviction bound `ArchiveTurn` enforces.
///
/// It doubles as the capacity reference for the context engine's fill ratio: the
/// dylib has no token counts and no context length, so the conversation index's
/// own cap is the closest available "full" reference (the index holds at most
/// this many turns).
pub(crate) const ConvIndexCap:usize = 50;
