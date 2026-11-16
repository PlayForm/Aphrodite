//! Resolve the live compression thresholds from env, TOML, then defaults.
//!
//! Called once at startup and again on every hot-reload, which is the whole
//! point: a config edit that only re-parsed and logged would leave the live
//! proxy on the old numbers. Precedence is env var > `[compression]` TOML
//! value > compiled-in default, the same order the listen-port override uses.

use crate::{
	Configuration::{Environment::Warn::Fn::Fn as Warn, Proxy::Compression::Struct::Struct as Table},
	Proxy::Threshold::{
		Const::{Cache, InitialExpand, Inline, Token},
		Struct::Struct as ResolvedThresholds,
	},
};

/// Resolve the compression thresholds and auto-expand knobs from env vars,
/// the TOML `[compression]` table (if any), and the compiled-in defaults,
/// in that precedence order.
///
/// `Multiplier` (`[compression] code_multiplier`) defaults to `3.0`, not the old
/// free-function's `2`: the prior default only existed because
/// `APHRODITE_CODE_MULTIPLIER=3.0` silently failed a `usize` parse (report 07
/// F10) - every shipped TOML and doc has always said `3.0`, so `2` was a masked
/// bug, not an intentional value, and this restores the value the project's own
/// config always claimed.
pub fn Fn(Compression:Option<&Table>) -> ResolvedThresholds {
	ResolvedThresholds {
		Cache:Warn::<usize>("APHRODITE_TOOL_THRESHOLD_CACHE")
			.or_else(|| Compression.and_then(|Table| Table.Cache).map(|Value| Value as usize))
			.unwrap_or(Cache),
		Token:Warn::<usize>("APHRODITE_TOOL_THRESHOLD_TOKEN")
			.or_else(|| Compression.and_then(|Table| Table.Token).map(|Value| Value as usize))
			.unwrap_or(Token),
		inline:Warn::<usize>("APHRODITE_INLINE_THRESHOLD")
			.or_else(|| Compression.and_then(|Table| Table.Inline).map(|Value| Value as usize))
			.unwrap_or(Inline),
		CodeMultiplier:Warn::<f64>("APHRODITE_CODE_MULTIPLIER")
			.or_else(|| Compression.and_then(|Table| Table.Multiplier))
			.unwrap_or(3.0),
		// Presence-aware env read: `APHRODITE_AUTO_EXPAND=0`/`false` must
		// override a TOML `auto_expand = true`, so a bare `Environment::Bool`
		// (absent => false) would be wrong here - absent must fall through
		// to the TOML value.
		AutoExpand:match std::env::var("APHRODITE_AUTO_EXPAND") {
			Ok(Value) => matches!(Value.to_lowercase().as_str(), "1" | "true"),
			Err(_) => Compression.and_then(|Table| Table.Expand).unwrap_or(false),
		},
		ExpandLimit:Warn::<usize>("APHRODITE_AUTO_EXPAND_LIMIT")
			.or_else(|| Compression.and_then(|Table| Table.Limit).map(|Value| Value as usize))
			.unwrap_or(InitialExpand),
		// Presence-aware: `APHRODITE_MAX_REQUEST_BODY_SIZE=0` is a legitimate
		// "reject every non-empty body" configuration, so absent-env must
		// fall through to TOML then to None (no cap) rather than parse 0.
		BodyLimit:Warn::<usize>("APHRODITE_MAX_REQUEST_BODY_SIZE").or_else(|| Compression.and_then(|Table| Table.Body)),
	}
}

// ── spawn_blocking wrappers for CcrStore (rusqlite is blocking) ─────
