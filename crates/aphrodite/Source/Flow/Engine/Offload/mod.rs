//! Context-engine archiving: trigger, gate, and section wrapper.
//!
//! Three pieces, kept apart because they fail differently: `Archive/` decides
//! whether the engine is consulted at all and whether anything was archived,
//! `Section/` decides how a `Some`/`None` answer becomes text, and `Block/`
//! sequences them. Collapsing them would make "the engine ran and archived
//! nothing" indistinguishable from "the engine was never asked".

pub mod Archive;
pub mod Block;
pub mod Section;
