//! `Resolve` tests, split by the knob family each one pins down.
//!
//! These encode the contract: environment > TOML (proxy over defaults) >
//! hardcoded default, per knob. They are moved, not rewritten - a weakened
//! assertion here would hide exactly the drift the chain exists to prevent.

mod Default;
mod Env;
mod Error;
mod Mode;
mod Port;
mod Timeout;
