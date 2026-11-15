//! The backgrounding decision, split by what each hook can observe.
//!
//! `Entry/` holds the two public predicates; `Command/` holds the prefix
//! table and the matcher they share. The split exists so the table has one
//! owner - a second copy of it is how a command starts backgrounding on the
//! pre-hook and not the post-hook.

pub mod Command;
pub mod Entry;
