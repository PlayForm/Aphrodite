//! Aphrodite internal state - mirrors plugins/aphrodite/_core/state.py
//! All session-scoped state lives here: inline store, conv index, markers,
//! counters.
//!
//! The tree is grouped by the concern that owns the data, not by the order the
//! fields appear in the struct:
//!
//! - `Struct.rs` - `AphroditeState` itself: the composition and its seed,
//!   nothing else.
//! - `Identity/` - the three identity types the state's records are keyed by.
//! - `Template/` - the `[templates]` section of the configuration.
//! - `Configuration/` - the resolved config the session mirrors.
//! - `ChainSplit/` - the Tier 1 teaching loop: events, attribution, adaptation.
//! - `Directive/` - loaded/active/ephemeral directives and the manual latch.
//! - `Event/` - the per-tool-call telemetry ring.
//! - `File/` - the referenced-files list.
//! - `Marker/` - the recent-marker catalog.
//! - `PollWorker/` - the background-task deque and its master switch.
//! - `Store/` - the inline content store: LRU order and byte budget.
//! - `Turn/` - the turn counter, the scan cursor and the conversation index.
//! - `Test/` - the battery, split by area.
//!
//! Each domain directory owns one `Struct` - that domain's share of the session
//! state - plus the operations that touch it. `AphroditeState` composes the
//! nine, so a function that needs one domain takes `&mut <Domain>::Struct`
//! instead of the whole state.
//!
//! Each bound lives with the structure it bounds (`Marker::Const::Cap`,
//! `Store::Entry::Const::Cap`, `Store::Const::Budget`, `ChainSplit::Const::Cap`),
//! so the file that enforces a cap is the file you edit to change it.
//!
//! Mutation ordering is a documented invariant, not an implementation detail:
//! `Store::Put` accounts bytes before the explicit eviction pass, and the LRU
//! cache is sized one larger than the entry cap so it never self-evicts behind
//! that pass. An interruption therefore degrades to "entry missing" rather than
//! "entry duplicated" or "byte total drifted". The `Test/` battery is the proof
//! of that ordering - it pins eviction order, the caps and the byte accounting.
//!
//! This facade declares the tree and re-exports the exact public names the
//! rest of the crate (and `aphrodite-hermes` via `Library.rs`) depends on.
//! Nothing else belongs in this file.

pub mod ChainSplit;
pub mod Configuration;
pub mod Directive;
pub mod Event;
pub mod File;
pub mod Identity;
pub mod Marker;
pub mod PollWorker;
pub mod Store;
pub mod Struct;
pub mod Template;
pub mod Turn;

#[cfg(test)]
pub mod Test;
