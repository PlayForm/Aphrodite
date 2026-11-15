//! Writing resolved configuration into runtime state.
//!
//! `Compression` is the single entry point (`ApplyCompression`), and it does
//! nothing but call the per-section appliers in their original order. The
//! split exists so a section can be read, changed and tested without touching
//! the other seven; it is not licence to reorder them, because a few sections
//! seed values the next one reads (`ChainSplitFloor` from
//! `ChainSplitMinimumSegments`, `ActiveDirectives` from the loaded set).
//!
//! `Global` is separate because those two appliers do not write state - they
//! write process globals that paths without an `AphroditeState` (hooks, the
//! dylib, the poll worker) read.

pub mod Compression;
pub mod Global;
pub mod Section;
