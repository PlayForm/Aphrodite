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
//!
//! This leaf only supplies the SINK; the decision lives in `At::Fn::Fn`, which is
//! what a probe can drive. Note that the core sink emits the record on its own
//! target (`aphrodite::debug`) as well as appending it, so a process that
//! enables both this crate's target and the core's sees the same record twice -
//! once structured, once as the formatted line. That is deliberate: the two
//! targets are the two audiences (this crate's bridge, and the engine), and
//! neither filter is guaranteed to be on.

use crate::Debug::{Init::Const::TARGET, Log::Bridge::At};

/// Bridge-side event: env level OR the session flag toggle.
///
/// It does not filter or truncate - a caller that logs a 40 KB value gets a 40 KB
/// line, so callers pass sizes and hashes rather than payloads, and no caller
/// here ever passes a secret or a content body.
pub(crate) fn Fn(state:&aphrodite::State::Struct::AphroditeState, session:&str, event:&str, Fields:&[(&str, String)]) {
	At::Fn::Fn(state, session, event, Fields, |Level, Event, Fields| {
		tracing::debug!(target: TARGET, session, event = Event, level = Level, fields = ?Fields);
		aphrodite::Debug::Log::At::Fn::Fn(Level, Event, Fields);
	});
}
