//! Re-reading one key from a configuration file: assess outside the lock, then
//! apply.
//!
//! The reload path used to read the file inside `Guarded`'s closure, holding
//! the global handle-map mutex across disk I/O - on a slow or cold mount one
//! handle's reload blocked every other handle's every call, and it was a trap
//! for any future closure that called back into another `aphrodite_*` export
//! (non-reentrant mutex, instant deadlock). The fix is the same shape the
//! setters use: everything that touches the filesystem or a parser runs first,
//! produces one value, and the lock is taken once to apply it.
//!
//! - `Assessment/` - the four situations a reload can be in.
//! - `Assess/` - the file read and parse, before the lock.
//! - `Apply/` - the state mutation, under the lock.

pub mod Apply;
pub mod Assess;
pub mod Assessment;
