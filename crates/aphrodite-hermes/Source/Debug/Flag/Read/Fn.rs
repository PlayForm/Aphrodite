//! The cached read of one flag file.
//!
//! The read has to tolerate the logger: the core debug logger APPENDS
//! timestamped lines to this same file, so the first line can be a flag value
//! fused with a log line (`on[2026-...]`). Hence a prefix match on the trimmed,
//! lowercased content rather than a whole-line comparison.

use std::{collections::HashMap, path::Path, sync::Mutex};

use crate::Debug::Flag::Cache::FLAG_CACHE;

/// Mtime-cached read of one flag file; missing file = off.
///
/// Enabled values are `1`, `on`, `true` and `debug` as a PREFIX of the file's
/// leading non-whitespace text; anything else is off. It is not a parser: a file
/// whose first line starts with `on` is on even if later lines say otherwise,
/// because the first line is the toggle and everything after it is history.
pub(crate) fn Fn(path:&Path) -> bool {
	let key = path.to_string_lossy().into_owned();
	let mtime = std::fs::metadata(path)
		.and_then(|m| m.modified())
		.ok()
		.map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0))
		.unwrap_or(0.0);
	let cache = FLAG_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
	let mut guard = cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	if let Some((cached_mtime, cached_on)) = guard.get(&key)
		&& *cached_mtime == mtime
	{
		return *cached_on;
	}
	let on = std::fs::read_to_string(path)
		.ok()
		// PREFIX match on the first token, NOT a whole-line match: the
		// env-gated logger (`aphrodite::debug`) APPENDS timestamped lines
		// to this same `debug.<root>` file, and a legacy flag file may have
		// no trailing newline (the toggle writes "on"/"off" bare), so the
		// first log line can merge into the flag value (e.g. "on[2026-...]").
		// A value-prefix check keeps the toggle readable in every case.
		.map(|s| s.trim_start().to_lowercase())
		.map(|s| s.starts_with("1") || s.starts_with("on") || s.starts_with("true") || s.starts_with("debug"))
		.unwrap_or(false);
	guard.insert(key, (mtime, on));
	on
}
