//! The referenced-files list.
//!
//! The list is what lets the turn context say "you already have this file" -
//! so it must reflect the most recent touch of each path, not the first. A
//! path is therefore removed and re-pushed on every reference, and the oldest
//! distinct path is evicted when the list is full.
//!
//! - `Struct.rs` - the list, the tool set and the delta counter.
//! - `Default.rs` - the seed: the default file-tool set, an empty list.
//! - `Record.rs` - record one reference, newest at the front.
//! - `Const.rs` - the list's cap.

pub mod Const;
pub mod Default;
pub mod Record;
pub mod Struct;
