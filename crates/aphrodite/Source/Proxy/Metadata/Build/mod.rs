//! `Build` - Build output: OK/FAIL, the files compiled, the first error.
//!
//! The item lives in `Fn.rs`; this facade only declares it.

pub mod Fn;
pub(crate) use Fn::Fn;
