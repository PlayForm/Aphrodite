//! The proxy port pair, resolved from the environment once per call.
//!
//! Two Hermes Agent instances on one machine each need their own proxy pair,
//! so the ports are overridable (`aphrodite setup --cache-port/--token-port`
//! writes them into the environment). A malformed override must be LOUD and
//! fall back to the default - silently ignoring it leaves an operator unable
//! to tell "my override didn't apply" from "I didn't set an override".
//!
//! This leaf is the environment read and nothing else: the resolution itself is
//! `Resolve::Fn::Fn`, which takes the lookup as an argument so it can be proved
//! without mutating the process environment.

use crate::Health::Port::Resolve;

/// Default cache proxy port, used when `APHRODITE_CACHE_PORT` is unset.
pub(crate) const Cache:u16 = 9797;
/// Default token proxy port, used when `APHRODITE_TOKEN_PORT` is unset.
pub(crate) const Token:u16 = 9798;

/// Resolve the cache/token proxy ports for this process.
///
/// Reads `APHRODITE_CACHE_PORT` / `APHRODITE_TOKEN_PORT` from the environment
/// so that multiple concurrent Hermes Agent instances on the same machine can
/// each be pointed at their own proxy pair (see `aphrodite setup --cache-port
/// / --token-port`), falling back to the historical 9797/9798 defaults.
pub(crate) fn Fn() -> (u16, u16) { Resolve::Fn::Fn(|Var| std::env::var(Var).ok()) }
