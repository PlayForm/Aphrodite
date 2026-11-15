//! `Code` - Code of unknown language: any signature-looking line, labelled `gen`.
//!
//! The item lives in `Fn.rs`; this facade only declares it.

pub mod Fn;
pub(crate) use Fn::Fn;
