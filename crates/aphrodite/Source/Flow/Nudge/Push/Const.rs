//! How many ephemeral nudges the queue may hold.

/// Maximum queued inline nudges.
///
/// A nudge is a one-shot correction of the model's current behaviour, not a second
/// directive channel: past four entries the oldest is dropped, so a caller that
/// pushes every turn cannot grow the turn's budget without bound.
pub(crate) const Maximum:usize = 4;
