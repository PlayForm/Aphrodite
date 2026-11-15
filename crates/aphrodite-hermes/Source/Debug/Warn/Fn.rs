//! The one warning that has to survive a missing subscriber.

use crate::Debug::Init::Const::TARGET;

/// Emit a diagnostic that reaches a log surface in every host.
///
/// `tracing::warn!` is a silent no-op while no subscriber is installed, and the
/// dylib's own install deliberately loses to a host that already owns the
/// dispatcher (see `Debug::Init`). Fall back to stderr in exactly that
/// unsubscribed case, so a degradation is never invisible - and only in that
/// case, so one warning never prints twice. The core crate's parse-failure warn
/// uses the same two-step for the same reason.
///
/// Callers pass the message WITHOUT the `aphrodite-hermes:` prefix: it is added
/// here, once, so every bridge warning is greppable by one string.
pub(crate) fn Fn(Message:&str) {
	tracing::warn!(target: TARGET, "{}", Message);

	// `eprintln!` is the fallback for a host that installed no subscriber, so it
	// must not also fire when one exists: `then` keeps it off that path.
	let _ = (!tracing::dispatcher::has_been_set()).then(|| eprintln!("aphrodite-hermes: {Message}"));
}
