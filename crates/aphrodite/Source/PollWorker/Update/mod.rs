//! Poll-result ingestion: decode, apply, then interpret.
//!
//! The two phases are load-bearing and now explicit. `Apply` mutates the registry
//! under its borrow; `Interpret` needs `&mut AphroditeState` to store the completed
//! output and record its marker. Doing it in one pass double-borrows the state,
//! which is what the old "phase 1 / phase 2" comments were working around.

pub mod Fn;
pub mod Interpret;
