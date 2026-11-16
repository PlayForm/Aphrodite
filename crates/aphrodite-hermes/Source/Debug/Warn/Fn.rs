//! The one warning that has to survive a host-owned dispatcher.

use crate::Debug::Init::{Const::TARGET, Ownership};

/// Emit a diagnostic that reaches a log surface in every host.
///
/// `tracing::warn!` is a silent no-op while no subscriber is installed, and the
/// dylib's own install deliberately loses to a host that already owns the
/// dispatcher (see `Debug::Init`). The fallback decision is therefore taken from
/// the ownership this crate RECORDED at install time, not from
/// `tracing::dispatcher::has_been_set()`:
///
///   - `has_been_set()` answers "is there a global subscriber", which proves
///     nothing about whether THIS event reaches a human. A host that owns the
///     dispatcher may filter this crate's target out entirely, and then the
///     warning is exactly as invisible as if no subscriber existed.
///   - the recorded decision is the fact we actually have: this crate installed
///     the dispatcher (its own fmt layer, target-restricted to this crate) or a
///     host did. Only the second case can swallow a warning, so only the second
///     case prints to stderr.
///
/// `None` - no install attempted yet - takes the same branch as `HostOwned`:
/// nothing is known to be listening, and an unheard warning is the worse error.
/// The cost of being wrong is one duplicated line when a host both owns the
/// dispatcher and logs our target; the cost of the old rule was a warning that
/// vanished.
///
/// Callers pass the message WITHOUT the `aphrodite-hermes:` prefix: it is added
/// here, once, so every bridge warning is greppable by one string.
pub(crate) fn Fn(Message:&str) {
	tracing::warn!(target: TARGET, "{}", Message);

	// `eprintln!` is the fallback for a dispatcher this crate does not own, so it
	// must not also fire when this crate installed one.
	let Reported = matches!(Ownership::Fn::Fn(), None | Some(Ownership::Enum::Enum::HostOwned));

	if Reported {
		eprintln!("aphrodite-hermes: {Message}");
	}
}
