//! Budget enforcement for the assembled context.
//!
//! `Fn.rs` is the pass itself, `Measured/` is what survived it, and `Outcome/` is
//! the result the caller has to handle - `Fits` or `ProtectedOverflow`.

pub mod Fn;
pub mod Measured;
pub mod Outcome;
