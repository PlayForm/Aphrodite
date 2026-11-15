//! The C ABI's single entry into provisioning: resolve, then materialize.
//!
//! The parent facade declares `Into` as `pub(crate)`, so the C ABI export
//! names `Directive::Into::Fn::Fn` directly.

pub mod Fn;

