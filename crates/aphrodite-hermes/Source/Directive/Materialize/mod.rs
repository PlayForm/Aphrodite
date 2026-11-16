//! Non-destructive write of the embedded set into a resolved directory.
//!
//! `Fn/` folds one outcome per file into the `Report/` value; `Write/` decides
//! that outcome and `Outcome/` names it.

pub mod Fn;
pub mod Outcome;
pub mod Report;
pub mod Write;
