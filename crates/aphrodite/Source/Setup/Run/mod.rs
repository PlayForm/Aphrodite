//! The setup run: its entry point, its error type, and the context it threads.
//!
//! `Entry/` decides the order of the install steps and is the only place that
//! does; `Context/` holds the two resolved homes plus the binary's identity;
//! `Error/` is what a failed step returns. The order is the contract - homes
//! before any write, prerequisites before any write at all, artifacts before the
//! loader, registration last - so it lives in one function rather than being
//! spread across the leaves.

pub mod Context;
pub mod Entry;
pub mod Error;
