//! One build-log line's class.

/// What a build-log line contributes to the reduction.
///
/// Four cases, and the fourth is a real value rather than a `None`: a `Compiling ...` line is
/// neither an error, a warning nor a summary, and saying so is what lets the reducer `match` one
/// value instead of running three predicates in a guard chain.
pub(crate) enum Enum {
	Error,
	Warning,
	Summary,
	Ignore,
}
