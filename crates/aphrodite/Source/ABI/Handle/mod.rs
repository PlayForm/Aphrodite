//! Session-handle plumbing: the process-global table, allocation, and lookup.
//!
//! A handle is an opaque decimal string the caller round-trips unchanged; it
//! names one `AphroditeState` living behind its own mutex inside
//! `Table::HANDLES`. Nothing here is exported to C - the exports that take a
//! handle are one file each beside this directory, and they all reach state
//! through `WithState` so the map lock is never held across a hook body.

pub mod Allocate;
pub mod Lookup;
pub mod Table;

pub(crate) use Allocate::AllocHandle;
pub(crate) use Lookup::{GetHandle, WithState};
pub(crate) use Table::Handles;
