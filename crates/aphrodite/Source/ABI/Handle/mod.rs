//! Session-handle plumbing: the process-global table, allocation, and lookup.
//!
//! A handle is an opaque decimal string the caller round-trips unchanged; it
//! names one `AphroditeState` living behind its own mutex inside
//! `Registry::Const::Handles`. Nothing here is exported to C - the exports that
//! take a handle are one file each beside this directory, and they all reach
//! state through `Lookup::WithState` or `Lookup::Locked`, so the table lock is
//! never held across a hook body.
//!
//! Two layers, deliberately separate: `Registry` owns the table (initialisation,
//! allocation, collision policy, poison recovery, lookup, removal), and `Lookup`
//! owns the per-session lock plus the panic boundary. An export reaches state
//! through both, in that order.

pub mod Lookup;
pub mod Registry;
