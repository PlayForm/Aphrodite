//! Compiled-in compression defaults and the two response-body caps.
//!
//! These are the floor of the threshold precedence chain: every value here can
//! be overridden at runtime, and each one documents why its number is what it
//! is. They are constants rather than config keys because they are the
//! fallback that must exist before any config file has been read.

/// Content size threshold for cache mode compression (8KB).
pub(crate) const CacheCompressThreshold:usize = 8192;

/// Content size threshold for aphrodite mode compression (1KB).
pub(crate) const TokenCompressThreshold:usize = 1024;

/// Default `[compression] auto_expand_limit` byte cap - content at or
/// below this size is resolved inline instead of marker-emitted when
/// auto-expand is enabled. Matches the Hermes-side `AphroditeState`
/// default in `state/mod.rs` so the two consumers agree out of the box.
pub(crate) const AutoExpandLimitDefault:usize = 102400;

/// Inline CCR threshold - content below this size is stored in the inline
/// HashMap instead of SQLite/in-memory CCR backends, avoiding round-trip
/// overhead.
pub(crate) const InlineCCRThreshold:usize = 256;

/// Chat Completions API path.
pub(crate) const ChatCompletionsPath:&str = "/v1/chat/completions";

/// Max body size cached in `ResponseCache` (1 MB). `ResponseCache` is
/// count-bounded (128 entries via LRU) but not byte-bounded (report 06 F7) -
/// without this, 128 large completions (e.g. big code-generation responses)
/// could hold well over 100 MB resident for a cache whose only purpose is
/// avoiding a repeat upstream round-trip on an identical request.
pub(crate) const ResponseCacheMaximumBodyBytes:usize = 1024 * 1024;

/// Non-streaming response body cap (report 02-T10). A single buffered
/// `.bytes().await` can exhaust memory if the upstream returns a huge body
/// (e.g. a full codebase in a single completion). Streamed (SSE) responses
/// bypass this limit entirely - they're chunked at the protocol level.
pub(crate) const ResponseMaximumBodyBytes:usize = 64 * 1024 * 1024; // 64 MB

/// Initial compression-ratio EMA (×100: 2.0×) - conservative startup value
/// that avoids a startup scale-up in `ThresholdFor`'s auto-tune. Single
/// source of truth for the `CompressionRatioEma` initializers in
/// `build_state` and the test fixtures (W2-8) so the two can never diverge
/// again.
pub(crate) const InitialRatioEma:u64 = 200;

/// Initial `FillPct` (×100), computed from `InitialRatioEma` exactly the
/// way `AppState::compute_fill_pct` does (90.00% at the 2.0× initial EMA) -
/// a const fn so the manual 9000/200 sync can never drift (W2-8).
pub(crate) const fn InitialFillPct() -> u64 {
	// compute_fill_pct's `ratio_ema == 0` → 99 branch is unreachable here:
	// the initial EMA is always non-zero. `Ord::clamp`/`max`/`min` are not const
	// on this toolchain, so the [1, 99] bound is expressed with `saturating_sub`:
	// the first line is `min(raw, 99)`, the second raises a zero to 1. Both are
	// `const fn`, so the value is a bound and not a branch.
	let raw = 100u64.saturating_sub(InitialRatioEma / 20);
	let Capped = raw - raw.saturating_sub(99);

	(Capped + 1u64.saturating_sub(Capped)) * 100
}
