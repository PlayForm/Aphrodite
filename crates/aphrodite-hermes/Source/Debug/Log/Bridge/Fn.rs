//! The bridge-side structured event.
//!
//! Everything the hook arms want to log goes through here so the flag-file
//! toggle and the env level are applied in exactly one place, and so one call
//! produces both surfaces an operator expects: a `tracing` event for the
//! process log (filtered by `RUST_LOG` / `APHRODITE_LOG`) and the same fields
//! appended to the per-session debug file (`debug.<root>`).
//!
//! The file half is a direct call, NOT a subscriber layer. The dylib may lose
//! the global-dispatcher race to its host (`Debug::Init`), and a layer-based
//! file sink would then stop producing the very file the flag toggle is read
//! from - the toggle would keep flipping and nothing would ever be written. The
//! two surfaces also answer different questions: the process log belongs to the
//! operator, the file belongs to the session tree.

use crate::Debug::{Init::Const::TARGET, Log::Level};

/// Bridge-side event: env level OR the session flag toggle.
///
/// `kv` is an ordered slice, not a map: the appended line keeps the caller's
/// field order, which is what makes two runs of the same event diffable. It
/// does not filter or truncate - a caller that logs a 40 KB value gets a 40 KB
/// line, so callers pass sizes and hashes rather than payloads, and no caller
/// here ever passes a secret or a content body.
pub(crate) fn Fn(state:&aphrodite::State::Struct::AphroditeState, session:&str, event:&str, kv:&[(&str, String)]) {
	let lvl = Level::Fn::Fn(state, session);

	match lvl {
		// Quiet path: no event, no file append, no formatting.
		0 => {},
		lvl => {
			tracing::debug!(target: TARGET, session, event, level = lvl, fields = ?kv);
			aphrodite::Debug::Log::At::Fn::Fn(lvl, event, kv);
		},
	}
}
