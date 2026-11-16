//! `LogAt`: append one structured record.

use std::fmt::Write as _;

use crate::Debug::{
	Log::{Emit::Fn::Fn as EmitDebugRecord, Sanitize::Fn::Fn as SanitizeKv},
	Path::Log::Fn::Fn as LogPath,
	Time::Fn::Fn as IsoTimestamp,
};

/// Append one timestamped structured line to `debug.<root>`. Best-effort:
/// any failure (missing dir, permissions) degrades to silence - never a
/// panic, never a broken turn.
///
/// The record is formatted and handed to `tracing` BEFORE the destination is
/// resolved, so an operator with `APHRODITE_LOG=aphrodite=debug` sees the
/// engine's decisions even in a process that has no root session yet (the file
/// needs a session id; the stream does not). The level-0 gate above still comes
/// first, so a quiet session formats nothing.
///
/// The fresh-line check before the append is load-bearing: the same file is
/// the legacy flag file, written as a bare "on"/"off" with no terminator, so a
/// raw append would merge the first log line into the flag value and flip the
/// toggle off. A debug logger must never change the behaviour it observes.
///
/// Every failure after the `tracing` emit is an early `return`, which is the
/// same silence the closure form produced: a missing root session, an
/// unopenable file and a failed seek all end the append without touching the
/// record already emitted.
pub fn Fn(Level:u8, Event:&str, Kv:&[(&str, String)]) {
	// A level of 0 is "off", which is exactly what `NonZeroU8` refuses to hold: the
	// record is formatted and emitted only for a level that exists.
	let Some(Level) = std::num::NonZeroU8::new(Level) else {
		return;
	};

	let mut Line = format!("[{}] [aphrodite-debug] lvl={Level} event={Event}", IsoTimestamp());

	// `write!` into the line the record already owns: one allocation for the
	// whole record instead of one per kv pair. Writing to a `String` cannot
	// fail, and the result is discarded the same way every other step of this
	// best-effort body is.
	let _ = Kv
		.iter()
		.try_for_each(|(Key, Value)| write!(Line, " {Key}={}", SanitizeKv(Value)));

	// The same record enters the `tracing` stack (`aphrodite::debug`, DEBUG),
	// so an operator reading one stream sees the engine's decisions beside the
	// proxy's spans. Emitted BEFORE the destination is resolved: a record that
	// cannot reach the file (no session id, missing dir, permissions) still
	// reaches the operator.
	EmitDebugRecord(Level.get(), &Line);

	let Some(path) = LogPath() else { return };

	Line.push('\n');

	// Best-effort: any failure (missing dir, permissions) degrades to
	// silence - a debug-logging failure must never break a turn.
	let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) else {
		return;
	};

	use std::io::{Read, Seek, SeekFrom, Write};

	// Fresh-line guarantee: a legacy flag file may lack a trailing newline
	// (the toggle writes "on"/"off" with no terminator), so a raw append
	// would merge the first log line into the flag value and flip the flag
	// off. The last byte is the value: a file that does not end with a newline
	// gets one, and every other case (no file, unreadable, already terminated)
	// gets an empty prefix - which `write_all` does not write at all.
	let LastByte = f
		.seek(SeekFrom::End(-1))
		.is_ok()
		.then(|| {
			let mut Last = [0u8; 1];

			f.read_exact(&mut Last).is_ok().then_some(Last[0])
		})
		.flatten();

	let Separator:&[u8] = match LastByte.filter(|Last| *Last != b'\n') {
		Some(_) => b"\n",
		None => b"",
	};

	let _ = f.write_all(Separator);

	let _ = f.write_all(Line.as_bytes());
}
