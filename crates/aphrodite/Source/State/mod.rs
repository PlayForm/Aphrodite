//! Aphrodite internal state - mirrors plugins/aphrodite/_core/state.py
//! All session-scoped state lives here: inline store, conv index, markers,
//! counters.
//!
//! The tree is grouped by the concern that owns the data, not by the order the
//! fields appear in the struct:
//!
//! - `Struct.rs` - `AphroditeState` itself, fields and their invariants only.
//! - `Default.rs` - the seed every session clones from.
//! - `Const.rs` - the caps and budgets the store and rings enforce.
//! - `Template/` - the `[templates]` section of the configuration.
//! - `ChainSplit/` - the Tier 1 teaching loop: events, attribution, adaptation.
//! - `Directive/` - ephemeral directive activations.
//! - `Event/` - the per-tool-call telemetry ring.
//! - `File/` - the referenced-files list.
//! - `Marker/` - the recent-marker catalog.
//! - `Store/` - the inline content store: LRU order and byte budget.
//! - `Test/` - the battery, split by area.
//!
//! Mutation ordering is a documented invariant, not an implementation detail:
//! `Store::Put` accounts bytes before the explicit eviction pass, and the LRU
//! cache is sized `INLINE_MAX + 1` so it never self-evicts behind that pass.
//! An interruption therefore degrades to "entry missing" rather than "entry
//! duplicated" or "byte total drifted". The `Test/` battery is the proof of
//! that ordering - it pins eviction order, the caps and the byte accounting.
//!
//! This facade declares the tree and re-exports the exact public names the
//! rest of the crate (and `aphrodite-hermes` via `Library.rs`) depends on.
//! Nothing else belongs in this file.

pub mod ChainSplit;
pub mod Const;
pub mod Default;
pub mod Directive;
pub mod Event;
pub mod File;
pub mod Marker;
pub mod Store;
pub mod Struct;
pub mod Template;

#[cfg(test)]
mod Test;

pub use ChainSplit::SplitEvent;
pub use Const::DefaultInlineByteBudget;
pub use Directive::ActiveDirective;
pub use Event::ToolEvent;
pub use Marker::MarkerEntry;
pub use Struct::AphroditeState;
pub use Template::TemplatesConfiguration;
