//! Whether a string field reads as prose or as an embedded JSON payload.
//!
//! Two envelope matchers ask the same question - an `error` or priority field
//! whose value is a nested object would preview as its own `{`/`[` fragment
//! instead of the payload it belongs to - so the test lives here once rather
//! than twice.

pub mod Fn;
