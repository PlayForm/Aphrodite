//! Reading a string argument out of a JSON args object.
//!
//! Every handler needs the same "missing key means empty string" rule, and it has
//! to be the same rule everywhere: a handler that defaulted to `None` instead
//! would need its own missing-field error, and the error text would drift per
//! tool.

pub mod Fn;

