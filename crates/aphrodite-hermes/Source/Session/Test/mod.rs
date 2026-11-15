//! Probes for the per-session registry and its bounded lock.
//!
//! These are the regression proofs for the gateway-wedge fix, and they are the
//! contract: session isolation (one session cannot see another's compressions)
//! and fail-open degrade (a wedged session returns within the bounded-lock
//! window instead of blocking). Do not weaken either.

mod Apply;
mod Debounce;
mod Isolation;
mod Wedged;
