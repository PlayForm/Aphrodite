//! Probes for the per-session registry, its bounded lock, and the config
//! watcher's apply/continue halves.
//!
//! These are the regression proofs for the gateway-wedge fix, and they are the
//! contract: session isolation (one session cannot see another's compressions),
//! fail-open degrade (a wedged session returns within the bounded-lock window
//! instead of blocking), and a reload sweep that reports what it reached instead
//! of answering a `bool`. Do not weaken any of them.

mod Apply;
mod Continue;
mod Debounce;
mod Isolation;
mod Wait;
mod Wedged;
