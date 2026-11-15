//! `aphrodite_debug` - the dev-only per-session debug toggle.
//!
//! Gated by the parent facade under `debug_assertions`: a release dylib that
//! registered it would fail `hermes plugins validate` as an undeclared tool.

mod Fn;

pub(crate) use Fn::Fn;
