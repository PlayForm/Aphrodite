//! `LogAt`: append one structured record.

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
/// first, so a quiet session formats nothing - `then` keeps the whole body in a
/// closure the quiet path never calls.
///
/// The fresh-line check before the append is load-bearing: the same file is
/// the legacy flag file, written as a bare "on"/"off" with no terminator, so a
/// raw append would merge the first log line into the flag value and flip the
/// toggle off. A debug logger must never change the behaviour it observes.
///
/// Every failure inside the body is a `return` from the closure rather than from
/// this function, which is the same silence the early returns produced: a
/// missing root session, an unopenable file and a failed seek all end the append
/// without touching the record already emitted to `tracing`.
pub fn Fn(Level:u8, Event:&str, Kv:&[(&str, String)]) {
	let _ = (Level != 0).then(|| {
		let mut Line = format!("[{}] [aphrodite-debug] lvl={Level} event={Event}", IsoTimestamp());

		Kv.iter().for_each(|(Key, Value)| {
			Line.push_str(&format!(" {Key}={}", SanitizeKv(Value)));
		});

		// The same record enters the `tracing` stack (`aphrodite::debug`, DEBUG),
		// so an operator reading one stream sees the engine's decisions beside the
		// proxy's spans. Emitted BEFORE the destination is resolved: a record that
		// cannot reach the file (no session id, missing dir, permissions) still
		// reaches the operator.
		EmitDebugRecord(Level, &Line);

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
		// off. Prefix a newline when the file does not already end with one.
		// `&&` keeps the short circuit: a failed read never inspects `Last`.
		let _ = f.seek(SeekFrom::End(-1)).is_ok().then(|| {
			let mut Last = [0u8; 1];
			let _ = (f.read_exact(&mut Last).is_ok() && Last[0] != b'\n').then(|| f.write_all(b"\n"));
		});

		let _ = f.write_all(Line.as_bytes());
	});
}
