//! The text of the parse-failure warning.

/// `Warn::Fn`'s message: what the operator is told when `aphrodite.toml` was
/// found but failed to parse.
///
/// Built separately from its emission so the message itself is testable
/// without installing a subscriber. A test that asserts through
/// `tracing::subscriber::with_default` is inherently racy: a callsite's
/// interest is cached process-wide, so the first test to reach this callsite
/// under the no-op subscriber can cache it as disabled and the capture then
/// never fires. Assert on this function, not on a captured event.
///
/// `Action` is the clause that follows the semicolon ("skipping", "using
/// defaults") - it tells the operator whether the loader kept looking or gave
/// up, which is the difference between "this file is ignored" and "everything
/// is on defaults".
pub(crate) fn Fn(path:&std::path::Path, Action:&str) -> String {
	format!("aphrodite.toml found but failed to parse; {} ({})", Action, path.display())
}
