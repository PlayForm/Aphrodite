//! Compiled-in compression defaults and the two response-body caps.
//!
//! These are the floor of the threshold precedence chain: every value here can
//! be overridden at runtime, and each one documents why its number is what it
//! is. They are constants rather than config keys because they are the
//! fallback that must exist before any config file has been read.
//!
//! Each name carries the word the resolved `Threshold::Struct` field carries -
//! `Cache`, `Token`, `Inline` - because this module is where their precedence
//! chain bottoms out; the caps say `Cap`, the initial values say `Initial`,
//! and the unit lives in the doc comment.

/// `[compression] tool_threshold_cache` (8 KB) - the size above which a body is
/// compressed in cache mode. Environment variable `APHRODITE_TOOL_THRESHOLD_CACHE`
/// overrides it.
pub(crate) const Cache:usize = 8192;

/// `[compression] tool_threshold_token` (1 KB) - the size above which a body is
/// compressed in token mode. Environment variable `APHRODITE_TOOL_THRESHOLD_TOKEN`
/// overrides it.
pub(crate) const Token:usize = 1024;

/// `[compression] auto_expand_limit` (100 KB) - content at or below this size is
/// resolved inline instead of marker-emitted when auto-expand is enabled.
/// Environment variable `APHRODITE_AUTO_EXPAND_LIMIT` overrides it. Matches the
/// Hermes-side
/// `AphroditeState` default in `state/mod.rs` so the two consumers agree out of
/// the box.
pub(crate) const InitialExpand:usize = 102400;

/// `[compression] inline_threshold` (256 B) - content below this size is stored
/// in the inline map instead of the SQLite/in-memory CCR backends, avoiding
/// round-trip overhead. Environment variable `APHRODITE_INLINE_THRESHOLD` overrides
/// it.
pub(crate) const Inline:usize = 256;

/// Byte cap on a body cached in `ResponseCache` (1 MB). The cache is
/// count-bounded (128 entries via LRU) but not byte-bounded (report 06 F7) -
/// without this, 128 large completions (e.g. big code-generation responses)
/// could hold well over 100 MB resident for a cache whose only purpose is
/// avoiding a repeat upstream round-trip on an identical request.
pub(crate) const CacheCap:usize = 1024 * 1024;

/// Byte cap on a non-streaming response body (64 MB, report 02-T10). A single
/// buffered `.bytes().await` can exhaust memory if the upstream returns a huge
/// body (e.g. a full codebase in a single completion). Streamed (SSE) responses
/// bypass this limit entirely - they are chunked at the protocol level.
pub(crate) const ResponseCap:usize = 64 * 1024 * 1024;

/// Initial compression-ratio EMA (×100: 2.0×) - conservative startup value that
/// avoids a startup scale-up in the threshold auto-tune. Single source of truth
/// for the `RatioEma` initializers in `build_state` and the test fixtures
/// (W2-8) so the two can never diverge again.
pub(crate) const InitialRatio:u64 = 200;

/// Initial `FillPct` (×100), computed from `InitialRatio` exactly the way
/// `AppState::Compute` does (90.00% at the 2.0× initial EMA) - a const fn so
/// the manual 9000/200 sync can never drift (W2-8). `0` is unreachable: the
/// initial EMA is never zero, so the fill is always inside `[1, 99]`.
pub(crate) const fn InitialFill() -> u64 {
	// `Ord::clamp`/`max`/`min` are not const on this toolchain, so the [1, 99]
	// bound is expressed with `saturating_sub`: the first line is `min(raw, 99)`,
	// the second raises a zero to 1. Both are `const fn`, so the value is a
	// bound and not a branch.
	let raw = 100u64.saturating_sub(InitialRatio / 20);
	let Capped = raw - raw.saturating_sub(99);

	(Capped + 1u64.saturating_sub(Capped)) * 100
}
