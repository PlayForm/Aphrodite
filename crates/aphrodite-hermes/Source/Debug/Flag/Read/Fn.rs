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
///
/// A fresh cache entry is the whole answer, so only a missing or stale one reads
/// the file: the quiet path costs one stat and one hash lookup.
pub(crate) fn Fn(Path:&Path) -> bool {
	let Key = Path.to_string_lossy().into_owned();
	let Mtime = std::fs::metadata(Path)
		.and_then(|Meta| Meta.modified())
		.ok()
		.map(|Time| {
			Time.duration_since(std::time::UNIX_EPOCH)
				.map(|Age| Age.as_secs_f64())
				.unwrap_or(0.0)
		})
		.unwrap_or(0.0);
	let Cache = FLAG_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
	let mut Guard = Cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

	let Cached = Guard
		.get(&Key)
		.filter(|(CachedMtime, _)| *CachedMtime == Mtime)
		.map(|(_, CachedOn)| *CachedOn);

	Cached.unwrap_or_else(|| {
		let On = std::fs::read_to_string(Path)
			.ok()
			// PREFIX match on the first token, NOT a whole-line match: the
			// env-gated logger (`aphrodite::debug`) APPENDS timestamped lines to
			// this same `debug.<root>` file, and a legacy flag file may have no
			// trailing newline (the toggle writes "on"/"off" bare), so the first
			// log line can merge into the flag value (e.g. "on[2026-...]").
			// A value-prefix check keeps the toggle readable in every case.
			.map(|Text| Text.trim_start().to_lowercase())
			.map(|Text| {
				Text.starts_with("1") || Text.starts_with("on") || Text.starts_with("true") || Text.starts_with("debug")
			})
			.unwrap_or(false);

		Guard.insert(Key, (Mtime, On));

		On
	})
}
