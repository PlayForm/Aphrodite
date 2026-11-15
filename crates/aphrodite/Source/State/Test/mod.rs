//! Test battery for the session state, split by the area it pins.
//!
//! The split is not cosmetic: `Store.rs` is the proof of the eviction and
//! byte-accounting ordering, `ChainSplit.rs` the proof of the teaching-loop
//! arithmetic, and `Marker.rs` / `File.rs` the proof of the two rings' caps.
//! A failure names the invariant that broke without needing the line number.

mod ChainSplit;
mod File;
mod Marker;
mod Store;
