//! The finished-task lifetime.

/// Turns a finished (or newly stale) task survives after it started.
///
/// This is the second clock: `StaleTurnAge` decides when a running task is
/// abandoned, this decides how long its record stays retrievable afterwards. The
/// value is the `16` the retention pass compared against before it had a name.
pub(crate) const FinishedLifetime:usize = 16;
