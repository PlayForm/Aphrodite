//! The proxy port pair, resolved from the environment once per call.
//!
//! Two Hermes Agent instances on one machine each need their own proxy pair,
//! so the ports are overridable (`aphrodite setup --cache-port/--token-port`
//! writes them into the environment). A malformed override must be LOUD and
//! fall back to the default - silently ignoring it leaves an operator unable
//! to tell "my override didn't apply" from "I didn't set an override".

/// Default cache proxy port, used when `APHRODITE_CACHE_PORT` is unset.
pub(crate) const DEFAULT_CACHE_PORT:u16 = 9797;
/// Default token proxy port, used when `APHRODITE_TOKEN_PORT` is unset.
pub(crate) const DEFAULT_TOKEN_PORT:u16 = 9798;

/// Resolve the cache/token proxy ports for this process.
///
/// Reads `APHRODITE_CACHE_PORT` / `APHRODITE_TOKEN_PORT` from the environment
/// so that multiple concurrent Hermes Agent instances on the same machine can
/// each be pointed at their own proxy pair (see `aphrodite setup --cache-port
/// / --token-port`), falling back to the historical 9797/9798 defaults.
pub(crate) fn configured_ports() -> (u16, u16) {
	// F11/F15: warn on a malformed (present but unparseable) value instead of
	// silently falling back - a missing var is unremarkable, but a typo'd one
	// left an operator with no way to tell "my override didn't apply" from "I
	// didn't set an override" (the exact bug class `apply_port_override`'s
	// comment in `aphrodite::config` documents).
	// This crate installs its own `tracing` subscriber (`Debug::Init`) but may
	// lose the global-dispatcher race to the host, so route the warning through
	// `Debug::Warn`: it logs via tracing AND falls back to stderr in exactly the
	// unsubscribed case. A typo'd port must never be invisible.
	let port_from_env = |var:&str, default:u16| {
		match std::env::var(var) {
			Ok(v) => {
				match v.parse::<u16>() {
					Ok(port) => port,
					Err(_) => {
						crate::Debug::Warn::Fn::Fn(&format!(
							"{}={:?} is not a valid port (1-65535); using default {}",
							var, v, default
						));
						default
					},
				}
			},
			Err(_) => default,
		}
	};
	(
		port_from_env("APHRODITE_CACHE_PORT", DEFAULT_CACHE_PORT),
		port_from_env("APHRODITE_TOKEN_PORT", DEFAULT_TOKEN_PORT),
	)
}
