//! `[compression] debug_level` - the dev-only per-session logger.

use super::super::super::Struct::Struct;
use crate::State::Struct::AphroditeState;

/// Capture `APHRODITE_DEBUG` into `state.debug_level` and mirror it into the
/// process-global level.
///
/// The mirror matters: stateless render points have no `AphroditeState` and
/// gate on the global, so setting the field without the mirror leaves them
/// logging at the wrong level. The value is clamped to 0..=2 - a level the
/// logger cannot honour is worse than a clamped one.
pub(crate) fn Apply(Configuration:&Struct, State:&mut AphroditeState) {
	// ── DEV-ONLY debug logger (temporary development instrumentation) ──
	// `APHRODITE_DEBUG=1|2` exported at Hermes launch turns the
	// per-session debug file logger on from turn 0 (see `crate::debug`;
	// the runtime `aphrodite_debug` tool stays for toggling). 0 = off
	// (default), 1 = on, 2 = verbose. Read here (like
	// `APHRODITE_AUTO_EXPAND` etc.) so the env var is captured into
	// state at session start; the process-global level mirror lets
	// stateless render points gate on the same flag. This is temporary
	// dev-only compute - never a release path.
	State.DebugLevel = Configuration.Number("APHRODITE_DEBUG", "compression", "debug_level", 0).min(2) as u8;
	crate::Debug::Level::Set::Fn::Fn(State.DebugLevel);
}
