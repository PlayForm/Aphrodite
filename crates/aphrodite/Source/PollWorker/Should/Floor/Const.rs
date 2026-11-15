//! The minimum size below which output is never backgrounded.

/// Bytes of output below which a result is passed through untouched.
///
/// The floor is about the agent's attention, not the command: a 200-byte result it
/// already has in full does not need a marker to poll.
pub(crate) const MinimumTrackedBytes:usize = 2048;
