//! `LogAt`: append one structured record.

use crate::Debug::{Log::Sanitize::Fn::Fn as SanitizeKv, Path::Log::Fn::Fn as LogPath, Time::Fn::Fn as IsoTimestamp};

/// Append one timestamped structured line to `debug.<root>`. Best-effort:
/// any failure (missing dir, permissions) degrades to silence - never a
/// panic, never a broken turn.
///
/// The fresh-line check before the append is load-bearing: the same file is
/// the legacy flag file, written as a bare "on"/"off" with no terminator, so a
/// raw append would merge the first log line into the flag value and flip the
/// toggle off. A debug logger must never change the behaviour it observes.
pub fn Fn(Level:u8, Event:&str, Kv:&[(&str, String)]) {
	if Level == 0 {
		return;
	}
	let Some(path) = LogPath() else { return };
	let mut Line = format!("[{}] [aphrodite-debug] lvl={Level} event={Event}", IsoTimestamp());
	for (Key, Value) in Kv {
		Line.push_str(&format!(" {Key}={}", SanitizeKv(Value)));
	}
	Line.push('\n');
	// Fresh-line guarantee: a legacy flag file may lack a trailing newline
	// (the toggle writes "on"/"off" with no terminator), so a raw append
	// would merge the first log line into the flag value and flip the flag
	// off. Prefix a newline when the file does not already end with one.
	let mut f = match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
		Ok(f) => f,
		// Best-effort: any failure (missing dir, permissions) degrades to
		// silence - a debug-logging failure must never break a turn.
		Err(_) => return,
	};
	use std::io::{Read, Seek, SeekFrom, Write};
	if f.seek(SeekFrom::End(-1)).is_ok() {
		let mut Last = [0u8; 1];
		if f.read_exact(&mut Last).is_ok() && Last[0] != b'\n' {
			let _ = f.write_all(b"\n");
		}
	}
	let _ = f.write_all(Line.as_bytes());
}
