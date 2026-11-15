//! Resolve the live compression thresholds from env, TOML, then defaults.
//!
//! Called once at startup and again on every hot-reload, which is the whole
//! point: a config edit that only re-parsed and logged would leave the live
//! proxy on the old numbers. Precedence is env var > `[compression]` TOML
//! value > compiled-in default, the same order the listen-port override uses.

use crate::Configuration::Environment::Warn::Fn::EnvironmentParseWarn;
use crate::Configuration::Proxy::Compression::Struct::Struct as CompressionConfiguration;
use crate::Proxy::Threshold::Const::AutoExpandLimitDefault;
use crate::Proxy::Threshold::Const::CacheCompressThreshold;
use crate::Proxy::Threshold::Const::InlineCCRThreshold;
use crate::Proxy::Threshold::Const::TokenCompressThreshold;
use crate::Proxy::Threshold::Struct::Struct as ResolvedThresholds;

/// Resolve the compression thresholds and auto-expand knobs from env vars,
/// the TOML `[compression]` table (if any), and the compiled-in defaults,
/// in that precedence order.
///
/// `CodeMultiplier` defaults to `3.0`, not the old free-function's `2`: the
/// prior default only existed because `APHRODITE_CODE_MULTIPLIER=3.0`
/// silently failed a `usize` parse (report 07 F10) - every shipped TOML and
/// doc has always said `3.0`, so `2` was a masked bug, not an intentional
/// value, and this restores the value the project's own config always
/// claimed.
pub fn Fn(Compression:Option<&CompressionConfiguration>) -> ResolvedThresholds {
	ResolvedThresholds {
		Cache:EnvironmentParseWarn::<usize>("APHRODITE_TOOL_THRESHOLD_CACHE")
			.or_else(|| Compression.and_then(|c| c.ToolThresholdCache).map(|v| v as usize))
			.unwrap_or(CacheCompressThreshold),
		Token:EnvironmentParseWarn::<usize>("APHRODITE_TOOL_THRESHOLD_TOKEN")
			.or_else(|| Compression.and_then(|c| c.ToolThresholdToken).map(|v| v as usize))
			.unwrap_or(TokenCompressThreshold),
		inline:EnvironmentParseWarn::<usize>("APHRODITE_INLINE_THRESHOLD")
			.or_else(|| Compression.and_then(|c| c.InlineThreshold).map(|v| v as usize))
			.unwrap_or(InlineCCRThreshold),
		CodeMultiplier:EnvironmentParseWarn::<f64>("APHRODITE_CODE_MULTIPLIER")
			.or_else(|| Compression.and_then(|c| c.CodeMultiplier))
			.unwrap_or(3.0),
		// Presence-aware env read: `APHRODITE_AUTO_EXPAND=0`/`false` must
		// override a TOML `auto_expand = true`, so a bare `EnvironmentBool`
		// (absent => false) would be wrong here - absent must fall through
		// to the TOML value.
		AutoExpand:match std::env::var("APHRODITE_AUTO_EXPAND") {
			Ok(v) => matches!(v.to_lowercase().as_str(), "1" | "true"),
			Err(_) => Compression.and_then(|c| c.AutoExpand).unwrap_or(false),
		},
		AutoExpandLimit:EnvironmentParseWarn::<usize>("APHRODITE_AUTO_EXPAND_LIMIT")
			.or_else(|| Compression.and_then(|c| c.AutoExpandLimit).map(|v| v as usize))
			.unwrap_or(AutoExpandLimitDefault),
		// Presence-aware: `APHRODITE_MAX_REQUEST_BODY_SIZE=0` is a legitimate
		// "reject every non-empty body" configuration, so absent-env must
		// fall through to TOML then to None (no cap) rather than parse 0.
		MaximumRequestBodySize:EnvironmentParseWarn::<usize>("APHRODITE_MAX_REQUEST_BODY_SIZE")
			.or_else(|| Compression.and_then(|c| c.MaximumRequestBodySize)),
	}
}

// ── spawn_blocking wrappers for CcrStore (rusqlite is blocking) ─────
