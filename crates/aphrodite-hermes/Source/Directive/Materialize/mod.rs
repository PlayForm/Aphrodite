//! Non-destructive write of the embedded set into a resolved directory.
//!
//! Re-exported under its kind name so the joining module can call it as
//! `Materialize::Fn()`.

mod Fn;

pub(crate) use Fn::Fn;
