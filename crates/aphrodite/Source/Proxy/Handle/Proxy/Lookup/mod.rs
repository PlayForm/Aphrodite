//! `Lookup` - //! The response-cache lookup on the request path.
//!
//! The item lives in `Fn.rs`; this facade only declares it.

pub mod Fn;
pub(crate) use Fn::Fn;
