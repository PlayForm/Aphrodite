//! The embedded directive set: the projection of the core crate's
//! `loaded_builtins()` that provisioning writes.
//!
//! The item is re-exported under its kind name so the write path can call it
//! as `Builtin::Fn()` - the module path carries the domain, the item carries
//! the kind.

mod Fn;

pub(crate) use Fn::Fn;
