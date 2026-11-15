//! Configuration writes and reloads: parse outside the lock, apply inside it.
//!
//! Both paths have the same shape and the same reason. A configuration write
//! reads a key and a value off the caller's pointers, decides what that means,
//! and only then touches session state; a reload reads and parses a file and
//! only then touches session state. In both cases the decision is a value
//! produced before the session lock is taken, so a slow parse never blocks
//! another session's call - and in the reload case, never holds the table lock
//! across disk I/O.
//!
//! - `Update/` - one configuration write, already parsed into its field's type.
//! - `Outcome/` - what became of a write, including the two ignored cases.
//! - `ParseUpdate/` - key and value text to `Update`, before the lock.
//! - `ApplyUpdate/` - `Update` to session state, under the lock.
//! - `Reload/` - the file assessment and its application.

pub mod ApplyUpdate;
pub mod Outcome;
pub mod ParseUpdate;
pub mod Reload;
pub mod Update;
