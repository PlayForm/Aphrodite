//! Port resolution: a malformed override falls back LOUDLY, a valid one wins.
//!
//! The lookup is supplied, so these probes never touch the process environment.
//! They used to `unsafe { std::env::set_var }` under a test mutex, which is
//! unsafe for a reason the mutex cannot fix: `setenv` races `getenv` against
//! every other thread in the harness, and the harness owns those threads.

use crate::Health::Port::{
	Fn::{DEFAULT_CACHE_PORT, DEFAULT_TOKEN_PORT},
	Resolve::Fn::Fn as resolve,
};

// ── T9 (F11/F15): a malformed override must fall back to the default, not
// propagate a parse failure or silently pick something else - and it must not
// panic. The token var is absent in the same probe, so both branches of the
// lookup are exercised. ──
#[test]
fn probe_malformed_port_falls_back_to_the_default() {
	let (Cache, Token) = resolve(|Var| {
		match Var {
			"APHRODITE_CACHE_PORT" => Some("not-a-port".to_string()),
			_ => None,
		}
	});

	assert_eq!(Cache, DEFAULT_CACHE_PORT);
	assert_eq!(Token, DEFAULT_TOKEN_PORT);
}

// ── Port 0 parses as a `u16` but is not a port anything can connect to, so it is
// rejected exactly like a malformed string rather than accepted and silently
// probed forever. ──
#[test]
fn probe_port_zero_falls_back_to_the_default() {
	let (Cache, _Token) = resolve(|Var| {
		match Var {
			"APHRODITE_CACHE_PORT" => Some("0".to_string()),
			_ => None,
		}
	});

	assert_eq!(Cache, DEFAULT_CACHE_PORT);
}

// ── A valid override wins, so the two-instances-on-one-machine case works. ──
#[test]
fn probe_valid_override_wins() {
	let (Cache, _Token) = resolve(|Var| {
		match Var {
			"APHRODITE_CACHE_PORT" => Some("19797".to_string()),
			_ => None,
		}
	});

	assert_eq!(Cache, 19797);
}
