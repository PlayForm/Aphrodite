//! Classify one log line.

use crate::Preview::Parse::GitLog::Event::Enum::Enum;

/// Classify a line as a record boundary or as an ordinary line.
///
/// The hash is trimmed and kept whole; the seven-character short form is a display
/// decision the renderer makes. It is not a hash validator - a `commit ` prefix is the
/// whole test, because the detector that routed content here already established that
/// the payload is a log.
pub(crate) fn Fn(Line:&str) -> Enum<'_> {
	match Line.strip_prefix("commit ") {
		Some(Rest) => Enum::Commit { Hash:Rest.trim() },
		None => Enum::Line { Trimmed:Line.trim() },
	}
}
