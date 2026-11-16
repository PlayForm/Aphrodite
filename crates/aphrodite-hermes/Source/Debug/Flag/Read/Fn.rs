//! The cached read of one flag file.
//!
//! The read has to tolerate the logger: the core debug logger APPENDS
//! timestamped lines to this same file, so the first line can be a flag value
//! fused with a log line (`on[2026-...]`). Hence a prefix match on the trimmed,
//! lowercased content rather than a whole-line comparison.

use std::{collections::HashMap, io::ErrorKind, path::Path, sync::Mutex};

use crate::Debug::Flag::{self, State};

/// Mtime-cached read of one flag file, as one of three states.
///
/// Enabled values are `1`, `on`, `true` and `debug` as a PREFIX of the file's
/// leading non-whitespace text; anything else in an existing file is off. It is
/// not a parser: a file whose first line starts with `on` is on even if later
/// lines say otherwise, because the first line is the toggle and everything after
/// it is history.
///
/// A fresh cache entry is the whole answer, so only a missing or stale one reads
/// the file: the quiet path costs one stat and one hash lookup.
///
/// The file read happens OUTSIDE the cache lock. The cache is process-global and
/// every session's flag check goes through it, so reading under it would put
/// every other session behind one disk read - the same class of mistake as
/// holding the registry lock across a TOML load. Two callers that both miss read
/// the file twice and store the same answer; that is cheaper than serialising
/// them.
pub(crate) fn Fn(Path:&Path) -> State::Enum::Enum {
	let Key = Path.to_string_lossy().into_owned();
	let Modified = std::fs::metadata(Path).and_then(|Meta| Meta.modified());
	let Mtime = Modified
		.as_ref()
		.ok()
		.and_then(|Time| Time.duration_since(std::time::UNIX_EPOCH).ok())
		.map(|Age| Age.as_secs_f64())
		.unwrap_or(0.0);

	let Cached = {
		let Map = Flag::Cache::Map.get_or_init(|| Mutex::new(HashMap::new()));
		let Guard = Map.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

		Guard.get(&Key).filter(|(Stamp, _)| *Stamp == Mtime).map(|(_, Flag)| *Flag)
	};

	Cached.unwrap_or_else(|| {
		let Fresh = match &Modified {
			Ok(_) => {
				match std::fs::read_to_string(Path) {
					Ok(Text) => {
						// PREFIX match on the first token, NOT a whole-line
						// match: the env-gated logger (`aphrodite::debug`)
						// APPENDS timestamped lines to this same
						// `debug.<root>` file, and a legacy flag file may have
						// no trailing newline (the toggle writes "on"/"off"
						// bare), so the first log line can merge into the flag
						// value (e.g. "on[2026-...]"). A value-prefix check
						// keeps the toggle readable in every case.
						let Text = Text.trim_start().to_lowercase();
						let Enabled = Text.starts_with("1")
							|| Text.starts_with("on")
							|| Text.starts_with("true")
							|| Text.starts_with("debug");

						match Enabled {
							true => State::Enum::Enum::On,
							false => State::Enum::Enum::Off,
						}
					},
					// The file exists (the stat above succeeded) but cannot be
					// read: permissions, or it was removed between the stat and
					// the read. Off is the safe answer - a logger that cannot
					// read its own switch must not turn itself on - and the
					// entry is cached under the real mtime, so the next touch is
					// picked up.
					Err(_) => State::Enum::Enum::Off,
				}
			},
			Err(Error) => {
				match Error.kind() {
					ErrorKind::NotFound => State::Enum::Enum::Absent,
					// Present but unstattable (a parent directory without
					// execute permission, a mount that went away): we cannot see
					// it, so we do not claim it is absent - we claim it is off,
					// which is the answer that keeps debug quiet.
					_ => State::Enum::Enum::Off,
				}
			},
		};

		let Map = Flag::Cache::Map.get_or_init(|| Mutex::new(HashMap::new()));
		let mut Guard = Map.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		Guard.insert(Key, (Mtime, Fresh));

		Fresh
	})
}
