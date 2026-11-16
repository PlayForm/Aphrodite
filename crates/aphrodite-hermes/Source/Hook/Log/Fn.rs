//! The dev-only session-init / config-summary line, emitted once per process.
//!
//! "Once per process" is deliberate and enforced by a static, not by the caller:
//! a gateway process serves many sessions and every one of them starts, so an
//! unconditional line would append a config dump per subagent spawn. The first
//! call that carries a real session id wins; the `last_session()` fallback is
//! NOT acceptable here (it can point at a stale persisted root from another
//! process and write the line into the wrong debug file).

use std::sync::atomic::{AtomicBool, Ordering};

use aphrodite::State::Struct::AphroditeState;

use crate::Health::Proxy::Fn::proxy_health;

/// Emitted once per process: the session-init/config-summary line (first
/// `on_session_start` or `pre_llm_call` that has a session id wins).
static INIT_EMITTED:AtomicBool = AtomicBool::new(false);

/// DEV-ONLY: emit the session-init/config-summary line exactly once per
/// process. Includes dylib version, root session id, the config summary
/// (thresholds, auto-expand, live container, recursion, directives,
/// templates), the proxy-side code_multiplier env override, and proxy
/// health.
pub(crate) fn Fn(State:&AphroditeState, Session:&str) {
	// `swap` still runs only for a real session (the `||` short-circuits), and
	// the line is emitted at most once per process.
	let Some(()) = (!(Session.is_empty() || INIT_EMITTED.swap(true, Ordering::Relaxed))).then_some(()) else {
		return;
	};

	let mut Fields = vec![
		("version", env!("CARGO_PKG_VERSION").to_string()),
		("session", Session.to_string()),
	];

	Fields.extend(aphrodite::Debug::Summary::Fn::Fn(State));

	// code_multiplier is proxy-side config; surface the env override only
	// (the dylib has no proxy thresholds loaded).
	if let Ok(Multiplier) = std::env::var("APHRODITE_CODE_MULTIPLIER") {
		Fields.push(("code_multiplier", Multiplier));
	}

	Fields.push(("proxy", proxy_health().to_string()));
	crate::Debug::Log::Bridge::Fn::Fn(State, Session, "init", &Fields);
}
