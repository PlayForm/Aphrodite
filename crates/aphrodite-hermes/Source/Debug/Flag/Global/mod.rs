//! The global (session-less) debug flag path.
//!
//! Applies only to calls with no session context; it is never a machine-wide
//! switch for sessions that have their own scoped flag.

mod Fn;

pub(crate) use Fn::Fn;
