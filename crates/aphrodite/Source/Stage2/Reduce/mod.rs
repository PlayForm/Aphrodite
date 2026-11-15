//! The Stage-2 reducers, one directory per content family.
//!
//! Each family is independent: a change to the diff summary cannot move a
//! JSON key list, and a new family is a new directory plus one registry line.
//! The families are grouped by what the content IS, not by which reducer
//! happens to be reused for a second type - the registry is what maps several
//! CCR types onto one family.

pub mod Build;
pub mod Code;
pub mod Diff;
pub mod JSON;
