//! The runtime home every debug artifact resolves against.
//!
//! One decision, shared by the flag files, the persisted session id and the
//! core logger's scratch home. Getting it wrong is silent: a flag written into
//! a shadow home is simply never read, and the toggle appears to do nothing.

pub mod Fallback;
pub mod Fn;
