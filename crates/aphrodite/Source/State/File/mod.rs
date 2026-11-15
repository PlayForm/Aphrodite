//! The referenced-files list.
//!
//! The list is what lets the turn context say "you already have this file" -
//! so it must reflect the most recent touch of each path, not the first. A
//! path is therefore removed and re-pushed on every reference, and the oldest
//! distinct path is evicted when the list is full.

pub mod Record;
