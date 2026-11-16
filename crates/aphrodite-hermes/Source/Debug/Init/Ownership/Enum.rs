//! Who owns the process-global `tracing` dispatcher, as this crate recorded it.

/// The logging ownership decision, made once at install time.
///
/// `libaphrodite_hermes` is a cdylib `dlopen`ed into a host process, so the
/// global dispatcher has exactly two possible owners and the difference decides
/// how a diagnostic has to be delivered:
///
///   - `Installed` - this crate won the install, and its fmt layer is
///     target-restricted to this crate, so an event of ours is guaranteed to
///     reach stderr;
///   - `Host` - the host installed its dispatcher first. Our events go
///     wherever the host routes them, which may be nowhere: a host filter that
///     does not name this crate's target drops them silently.
///
/// `tracing::dispatcher::has_been_set()` cannot distinguish the second case from
/// the first, which is why the decision is recorded here at the moment it is
/// made instead of being probed later.
pub(crate) enum Enum {
	/// This crate installed the process-global subscriber.
	Installed,
	/// A host already owned the dispatcher; this crate logs through it. The
	/// host owns the dispatcher, so this crate's events may be filtered away.
	Host,
}
