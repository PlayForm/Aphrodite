//! Resolve the cache/token proxy port pair from a supplied lookup.

use crate::Health::Port::Fn::{DEFAULT_CACHE_PORT, DEFAULT_TOKEN_PORT};

/// Resolve the cache/token proxy ports from `Lookup`, falling back to the
/// historical 9797/9798 defaults.
///
/// The lookup is a parameter rather than `std::env::var` because the resolution
/// is the thing worth proving and the environment is the thing worth not
/// touching: `std::env::set_var` is unsafe in edition 2024 - `getenv` races
/// `setenv` against every other thread in the process, and a test mutex cannot
/// exclude the rest of the harness. A probe passes a fixed map; production
/// passes the environment, once, in `Port::Fn`.
///
/// A malformed override must be LOUD and fall back to the default: silently
/// ignoring it leaves an operator unable to tell "my override didn't apply" from
/// "I didn't set an override". Port 0 is rejected with the same warning - it
/// parses as a `u16`, but nothing can connect to it, and the message has always
/// claimed the range is 1-65535.
///
/// The warning goes through `Debug::Warn`, which knows whether this crate owns
/// the dispatcher: a typo'd port must never be invisible.
pub(crate) fn Fn(Lookup:impl Fn(&str) -> Option<String>) -> (u16, u16) {
	let Port = |Var:&str, Default:u16| {
		match Lookup(Var) {
			Some(Value) => {
				match Value.parse::<u16>().ok().filter(|Port| *Port != 0) {
					Some(Port) => Port,
					None => {
						crate::Debug::Warn::Fn::Fn(&format!(
							"{}={:?} is not a valid port (1-65535); using default {}",
							Var, Value, Default
						));

						Default
					},
				}
			},
			None => Default,
		}
	};

	(
		Port("APHRODITE_CACHE_PORT", DEFAULT_CACHE_PORT),
		Port("APHRODITE_TOKEN_PORT", DEFAULT_TOKEN_PORT),
	)
}
