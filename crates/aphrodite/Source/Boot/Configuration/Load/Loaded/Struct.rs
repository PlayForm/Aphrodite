//! What a successful load produced.

use super::super::super::Source;

/// The configuration the process runs on, plus the log-format flag.
///
/// `LogCompact` is resolved at load time rather than from the document's own
/// resolve step, because the tracing subscriber has to be installed before that
/// runs - otherwise the diagnostics `MultiConfiguration::Resolve` emits (mode
/// fallback, port overrides, timeout clamping) fire against no registered
/// subscriber and are silently dropped.
pub(crate) struct Struct {
	/// The single source of truth for every listener.
	pub Source:Source::Enum::Enum,
	/// Whether the compact log format was requested.
	pub LogCompact:bool,
}
