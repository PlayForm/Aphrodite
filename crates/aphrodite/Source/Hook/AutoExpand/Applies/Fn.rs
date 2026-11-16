//! The auto-expand gate.

use crate::State::Struct::AphroditeState;

/// Auto-expand gate (dylib-side consumer of `[compression] auto_expand`):
/// when enabled AND the content fits `AutoExpandLimit` bytes, the
/// transform hooks return the resolved content inline instead of the CCR
/// marker, so no `<<<CCR:...>>>` marker reaches the model. Over the cap (or
/// disabled) the caller keeps today's marker behavior unchanged.
/// `limit == 0` is effectively off (no non-empty content fits), matching the
/// pre-Rust `AUTO_EXPAND_LIMIT = 0` = OFF semantics.
///
/// It is not a compression decision - it never decides whether to store
/// anything, only whether the stored result is returned inline - so the
/// transform hooks evaluate it before every marker-emitting gate.
pub(crate) fn Fn(State:&AphroditeState, Content:&str) -> bool {
	State.Configuration.AutoExpand && Content.len() <= State.Configuration.AutoExpandLimit
}
