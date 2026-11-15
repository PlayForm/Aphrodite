//! Turn-telemetry signatures: how a repeated call or a repeated error is
//! recognised as the same one.
//!
//! `Const` owns the volatile-key list, `Normalize` the argument hashing,
//! `Error` the error hashing. They share the hash primitive in `Flow::Hash` and
//! nothing else - a change to one must not move the other's numbers.

pub mod Const;
pub mod Error;
pub mod Normalize;

pub use self::{Error::Fn::ErrorSig, Normalize::Fn::NormalizeArgsSig};
