//! How a burst ended.

/// Why the debounce stopped waiting.
///
/// A burst ends in two ways that the caller has to tell apart: the file went quiet,
/// so a reload is the right response and the watcher keeps running, or the channel
/// closed, so this is the last burst this controller will ever see. A `()` return
/// collapsed both into "the wait is over", which left the loop re-receiving from a
/// closed channel - an immediate `None`, no suspension point, one core burned - and
/// it is why the loop's own `while let` had to be the thing that noticed.
pub(crate) enum Enum {
	/// The burst settled: the file has been quiet for the debounce window, or the
	/// ceiling expired first.
	Settled,
	/// The event source closed while the burst was still being drained.
	Closed,
}
