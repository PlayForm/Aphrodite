//! The security-boundary battery, one area per file.
//!
//! These exercise the pure decision functions the middlewares delegate to, so
//! they are the proof that the loopback, `Host` and bearer rules still hold -
//! not a reimplementation of them. A test that reimplements the rule instead of
//! calling it would keep passing after the rule was weakened.

mod Bearer;
mod Host;
mod Loopback;
