//! `WarnParseFailure` - the parse-failure warning that always lands.

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
/// defaults") - it tells the operator whether the loader kept looking or gave
/// up, which is the difference between "this file is ignored" and "everything
/// is on defaults".
/// The text of the parse-failure warning.
///
/// Built separately from its emission so the message itself is testable
/// without installing a subscriber. A test that asserts through
/// `tracing::subscriber::with_default` is inherently racy: a callsite's
/// interest is cached process-wide, so the first test to reach this callsite
/// under the no-op subscriber can cache it as disabled and the capture then
/// never fires. Assert on this function, not on a captured event.
pub(crate) fn ParseFailureMessage(path:&std::path::Path, Action:&str) -> String {
	format!("aphrodite.toml found but failed to parse; {} ({})", Action, path.display())
}

pub(crate) fn WarnParseFailure(path:&std::path::Path, error:&toml::de::Error, Action:&str) {
	let Message = ParseFailureMessage(path, Action);

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
