//! The parse-failure warning that always lands.

use super::Message;

/// Emit a parse-failure warning that actually reaches a log surface.
///
/// `tracing::warn!` is a silent no-op while no subscriber is installed -
/// exactly the Hermes dylib path (the host is a Python process that never
/// installs a Rust tracing subscriber) and the engine binary before
/// `main()` initializes one (config loads first). Fall back to stderr so a
/// found-but-broken config is never invisible. The caller additionally
/// records the failure on the returned `Configuration` (`ParseFailure`) for the
/// `AphroditeStats` self-diagnosis surface.
///
/// `Action` is the clause that follows the semicolon ("skipping", "using
/// defaults"); `Message::Fn::Fn` builds the text and explains why the clause
/// matters.
pub(crate) fn Fn(path:&std::path::Path, error:&toml::de::Error, Action:&str) {
	let Message = Message::Fn::Fn(path, Action);

	tracing::warn!(path = %path.display(), error = %error, "{}", Message);

	// Emit to stderr only when no GLOBAL subscriber is installed. The warn above
	// is already a no-op in that case, so this cannot double-report. Deciding by
	// `has_been_set()` alone (as this used to) is wrong for a thread-local
	// dispatcher - `with_default` does not set the global flag, so a caller
	// holding one saw the warning routed to stderr and never reached its own
	// subscriber.
	let _ = (!tracing::dispatcher::has_been_set())
		.then(|| crate::Logging::Stderr::Fn::Fn(&format!("[aphrodite] {Message}")));
}
