//! The Tier 1 teaching loop - consequence-driven chain-split thresholds.
//!
//! Chain splitting turns one giant tool output into N segment markers. Whether
//! that helps is not knowable up front: it depends on whether the agent
//! actually retrieves the segments. This module records what was produced,
//! attributes each later resolve back to the event that produced it, and moves
//! the minimum-segment threshold within its bounds from the retrieval ratio.
//!
//! The threshold is machinery. Nothing here is rendered to the LLM: no
//! directive text changes, no summary changes, no marker changes. The agent
//! learns from the consequence (a marker it did or did not need), never from
//! an instruction.
//!
//! - `Struct.rs` - the switch, the three bounds, the ledger and the map.
//! - `Default.rs` - the seed: off, floor 2, ceiling 6, empty ledger.
//! - `Entry/` - one event's consequence record (the item).
//! - `Record.rs` - record a split and register its segment hashes.
//! - `Retrieve.rs` - attribute a resolve to its event, once.
//! - `Adapt.rs` - the ratio rule and its bounds.
//! - `Const.rs` - the ledger's ring cap.
//! - `Window/` - how much evidence the adaptation requires.

pub mod Adapt;
pub mod Const;
pub mod Default;
pub mod Entry;
pub mod Record;
pub mod Retrieve;
pub mod Struct;
pub mod Trend;
pub mod Window;
