//! The setup run: its entry point and the context it threads.
//!
//! `Entry/` decides the order of the install steps and is the only place that
//! does; `Context/` holds the two resolved homes plus the binary's identity.
//! The order is the contract - homes before any write, prerequisites before any
//! write at all, artifacts before the loader, registration last - so it lives in
//! one function rather than being spread across the leaves.
//!
//! There is no setup-specific error enum: every step fails with `anyhow::Error`
//! whose context names the operation and the path, command or input that failed.
//! No caller ever branched on a setup failure kind - `Boot::Fn::Fn` prints the
//! chain and exits - so a variant hierarchy would have been an unread taxonomy.

pub mod Context;
pub mod Entry;
