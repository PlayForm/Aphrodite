//! Terminal-output compression.
//!
//! The same store-and-marker contract as tool results, with two differences
//! the shape of terminal output forces: marked output (chain-split segments
//! produced by a shell script) is split before any threshold gate can let it
//! leak raw, and a failing command is recorded even when its output is tiny -
//! `Inner/` is the gate order, the rest are the pieces it calls.

pub mod ChainSplit;
pub mod Emit;
pub mod Entry;
pub mod Inner;
pub mod Record;
