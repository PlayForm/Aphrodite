//! `[compression]` runtime toggles and caps - what the engine does while it
//! runs, as opposed to how much it compresses.

use super::super::super::Struct::Struct;
use crate::State::Struct::AphroditeState;

/// Write the poll-worker, auto-expand, live-container, recursion, body-size
/// and auto-reload switches.
///
/// Every one of these defaults OFF except the poll worker: they each add
/// behaviour the operator has to opt into, and a default-on surprise here is
/// indistinguishable from a bug.
pub(crate) fn Apply(Configuration:&Struct, State:&mut AphroditeState) {
	// ── Poll-worker auto-backgrounding ──
	State.PollWorkerEnabled = Configuration.GetBool("APHRODITE_POLL_WORKER", "compression", "poll_worker", true);
	// ── Auto-expand (pre-Rust port): resolve markers inline when enabled ──
	State.AutoExpand = Configuration.GetBool("APHRODITE_AUTO_EXPAND", "compression", "auto_expand", false);
	State.AutoExpandLimit =
		Configuration.GetUsize("APHRODITE_AUTO_EXPAND_LIMIT", "compression", "auto_expand_limit", 102400);
	// ── Live containers (pre-Rust port): streaming tool-output mode ──
	State.LiveContainer = Configuration.GetBool("APHRODITE_LIVE_CONTAINER", "compression", "live_container", false);
	// ── Caps (pre-Rust port): recursion depth + request-body limit ──
	State.RecursiveDepth = Configuration.GetUsize("APHRODITE_RECURSIVE_DEPTH", "compression", "recursive_depth", 5);
	State.MaximumRequestBodySize =
		Configuration.GetOptUsize("APHRODITE_MAX_REQUEST_BODY_SIZE", "compression", "max_request_body_size");
	// Opt-in config auto-reload (default OFF): when true, the dylib
	// watches aphrodite.toml and re-applies config fields on change -
	// never session CCR state (the dropped dylib-binary hot-reload
	// wiped it; fa85dfa). Env: `APHRODITE_AUTO_RELOAD`.
	State.AutoReload = Configuration.GetBool("APHRODITE_AUTO_RELOAD", "compression", "auto_reload", false);
}
