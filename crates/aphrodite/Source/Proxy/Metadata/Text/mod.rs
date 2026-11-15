//! `Text` - Plain text: the line count, and the fallback for every unclassified type.
//!
//! The item lives in `Fn.rs`; this facade only declares it.

pub mod Fn;
pub(crate) use Fn::Fn;
