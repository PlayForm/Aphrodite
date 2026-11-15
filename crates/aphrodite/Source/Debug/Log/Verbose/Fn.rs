//! `log_verbose`: a record only at level 2.

use crate::Debug::{Level::Get::Fn::Fn as Level, Log::At::Fn::Fn as LogAt};

/// Verbose-only line (level >= 2).
///
/// The gate is checked before any formatting or file I/O: the caller's kv pairs
/// are already built at the call site, but the timestamp, the record assembly
/// and the append are all skipped. That is what keeps a level-1 session paying
/// one atomic load per verbose call site instead of a line per call.
pub fn Fn(Event:&str, Kv:&[(&str, String)]) {
	match Level() >= 2 {
		true => LogAt(2, Event, Kv),
		false => (),
	}
}
