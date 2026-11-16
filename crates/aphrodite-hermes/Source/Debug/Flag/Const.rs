//! The flag-file basename.
//!
//! One word, shared by both path forms: the scoped flag is `debug.<session>` and
//! the global fallback is `debug`. Kept as a constant so a rename cannot leave
//! one of the two behind.

/// Basename of a debug flag file: the scoped form appends `.<session_id>`.
///
/// The value is the on-disk file name (`debug`), which is an external contract
/// with the core logger and the operator's `echo on >` command: it does not
/// follow a rename.
pub(crate) const Prefix:&str = "debug";
