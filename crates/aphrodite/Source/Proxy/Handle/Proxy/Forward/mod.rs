//! `Forward` - //! Send the upstream request, with the connect-only retry policy.
//!
//! The item lives in `Fn.rs`; this facade only declares it.

pub mod Fn;
pub(crate) use Fn::Fn;
