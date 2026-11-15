//! The bridge-side structured log line.
//!
//! Everything the hook arms want to log goes through here so the flag-file
//! toggle and the env level are applied in exactly one place. The line lands in
//! the core logger's per-root file, which is the same file the toggle is read
//! from - hence the prefix-tolerant flag reader next door.

use crate::Debug::Log::Level;

/// Bridge-side log line: env level OR the session flag toggle.
///
/// `kv` is an ordered slice, not a map: the emitted line keeps the caller's
/// field order, which is what makes two runs of the same event diffable. It
/// does not filter or truncate - a caller that logs a 40 KB value gets a 40 KB
/// line, so callers pass sizes and hashes rather than payloads.
pub(crate) fn Fn(state:&aphrodite::State::AphroditeState, session:&str, event:&str, kv:&[(&str, String)]) {
	let lvl = Level::Fn::Fn(state, session);
	if lvl > 0 {
		aphrodite::Debug::Log::At::Fn::Fn(lvl, event, kv);
	}
}
