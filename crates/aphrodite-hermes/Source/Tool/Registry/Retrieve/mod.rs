//! `aphrodite_retrieve` - resolve a CCR hash (recursively) or read a path.
//!
//! `Target/` is the argument decision as a type (path XOR hash), `Narrow/` is the
//! `query` filter both sources share, and `Fn/` is the handler.

pub mod Fn;
pub mod Narrow;
pub mod Target;
