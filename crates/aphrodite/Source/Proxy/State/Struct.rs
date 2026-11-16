//! Shared proxy state: upstream client config, CCR backend, counters, caches.
//!
//! Cloned as `Arc<AppState>` into every axum handler. The field set is split
//! into three disciplines - write-once configuration, live-tunable atomics,
//! monotonic counters - plus a handful of mutex-guarded collections; the doc
//! comment on the type is the contract for which group a field belongs to.

use std::{
	collections::VecDeque,
	sync::{
		Arc,
		atomic::{AtomicBool, AtomicU64, AtomicUsize},
	},
};

use reqwest::Client as HttpClient;
use headroom_core::ccr::CcrStore;
use tokio_util::task::TaskTracker;

use crate::{
	Configuration::Proxy::Mode::Enum::Enum as ProxyMode,
	Proxy::{
		Cache::{Entry::Struct::Struct as CacheEntry, Identity::Struct::Struct as CacheIdentity},
		Secret::Struct::Struct as Secret,
	},
};

/// Shared proxy state: upstream client config, CCR backend, and all
/// counters/caches used by request handlers. Wrapped in `Arc` and cloned
/// into every axum handler.
///
/// The 47 public fields fall into three discipline groups:
/// - **Write-once configuration**: `Client`, `StreamClient`, `APIURL`,
///   `Model`, `APIKey`, `ccr`, `AddMarkers`, `Mode`, `ToolRelay`,
///   `NotifyURL`, `NotifyKey`, `Dev`, `ResponseCacheTtl`,
///   `TaskTracker` - set in `build_state` and never mutated afterward
///   (hot-reload re-writes the atomic threshold fields below, not these).
/// - **Live-tunable atomics**: `CacheCompressThreshold`,
///   `TokenCompressThreshold`, `InlineCCRThreshold`,
///   `CodeMultiplierX100`, `AutoExpand`, `AutoExpandLimit`
///   (re-written by config reload), plus
///   `CompressionRatioEma` and `FillPct` (re-written by the compression
///   feedback loop). These are the only fields that change in place after
///   startup.
/// - **Monotonic counters**: `RequestsTotal`, `RequestsCompressed`,
///   `TokensSaved`, `CCRHits`, `CCRMisses`, `CCRCreated`,
///   `ToolRelayCalls`, `CacheHits`, `CacheMisses`, `InlineCCRHits`,
///   `InlineCCRMisses`, `ToolRelaySuccess`, `ToolRelayFailure`,
///   `NotifySuccess`, `NotifyFailure`, `UpstreamErrors4xx`,
///   `UpstreamErrors5xx`, `UpstreamTimeouts`, `UpstreamConnectErrors`,
///   `SSEStreamErrors`, `RequestBodyBytes`, `ResponseBodyBytes`,
///   `UpstreamLatencyMicros`, `TotalLatencyMicros` - only ever
///   incremented via `fetch_add`, never reassigned; `LatencyBuckets` is a
///   fixed histogram of the same class.
///
/// The remaining lock-guarded fields (`RequestHistory`, `InlineCCR`,
/// `LastErrors`, `CompressionsByType`, `ResponseCache`,
/// `UpstreamHealthCache`) are mutable state accessed only under their
/// mutex.
///
/// Discipline: every field belongs to exactly one group; moving one (or
/// adding a new one) requires deliberate review. The structural split into
/// a write-once `AppConfig` plus an `AppCounters` struct is deliberately
/// deferred to a dedicated refactor pass.
pub struct Struct {
	pub Client:HttpClient,
	/// 02-F2: a separate client with no total `.timeout()`, used only for
	/// requests the caller marked `"stream": true` before sending - reqwest's
	/// client-level timeout bounds the WHOLE request lifetime including
	/// consuming the response body, not just headers, so a chat completion
	/// streaming tokens past `cli.timeout` (300s default) had its
	/// `bytes_stream()` killed mid-answer on the shared, bounded `Client`.
	/// Hang protection here comes from `connect_timeout` + `tcp_keepalive`
	/// instead of a total deadline - deliberate, since a legitimately slow
	/// but still-progressing stream must not be cut off.
	pub StreamClient:HttpClient,
	pub APIURL:String,
	pub Model:String,
	pub APIKey:Secret,
	pub ccr:Option<Arc<dyn CcrStore>>,
	pub AddMarkers:bool,
	pub Mode:ProxyMode,
	pub ToolRelay:bool,
	pub NotifyURL:Option<String>,
	pub NotifyKey:Option<String>,
	/// Dev mode - verbose logging.
	pub Dev:bool,

	// Structured debug
	/// Ring buffer of last 50 request summaries
	/// Lock uses `.lock().map(...).unwrap_or_default()` - poison is safely
	/// tolerated: a poisoned mutex returns Err, and unwrap_or_default gives
	/// an empty/logical-default so the proxy stays up.
	pub RequestHistory:std::sync::Mutex<VecDeque<serde_json::Value>>,
	/// Inline CCR for tiny entries - no round-trip needed (< INLINE_CCR_THRESHOLD
	/// bytes). Lock uses `.lock().map(...)` - same poison safety pattern.
	/// Bounded to 1024 entries via LruCache to prevent unbounded memory growth.
	/// std::sync::Mutex rule: a held guard makes the future non-Send, so this
	/// lock is NEVER held across an `.await`; any future code that must hold
	/// it across one switches this field to `tokio::sync::Mutex`.
	pub InlineCCR:std::sync::Mutex<lru::LruCache<String, String>>,

	// Stats
	/// Latency histogram buckets (microseconds): 1ms, 10ms, 100ms, 1s, 10s
	pub LatencyBuckets:[AtomicU64; 5],
	/// Running total latency in microseconds for Prometheus _sum
	pub TotalLatencyMicros:AtomicU64,
	/// Track last N errors for hot-path analysis
	/// Mapped through `.lock().map(...)` - a poisoned lock returns Err and
	/// unwrap_or_default provides an empty Vec so error recording degrades
	/// gracefully without crashing the proxy.
	pub LastErrors:std::sync::Mutex<VecDeque<String>>,
	/// Compression decision counters by content type
	/// Uses `.lock().map(...)` - poison tolerant by design.
	pub CompressionsByType:std::sync::Mutex<std::collections::HashMap<String, u64>>,

	// Stats
	pub RequestsTotal:AtomicU64,
	pub RequestsCompressed:AtomicU64,
	/// Cumulative CONTENT bytes saved by compression, despite the name
	/// (report 05 F5: the field is exposed as `TokensSaved` in `/stats` and
	/// that external API name is kept for compatibility, but every call site
	/// accumulates raw BYTES - the removed content length minus the rendered
	/// marker that replaced it - never a token estimate. Previously one site
	/// divided by 4 to estimate tokens while every other site counted bytes,
	/// making the counter internally inconsistent by 4x; another subtracted the
	/// bare 40-char hash length instead of the actual (much longer) marker
	/// length, overstating savings.
	///
	/// A cache hit used to add its whole serialised response body here, which is
	/// an ENCODED-body measurement and now has its own counter
	/// (`BodyBytesSaved`): a hit replaces no content at all.
	pub TokensSaved:AtomicU64,
	/// Cumulative encoded response-body bytes a cache hit avoided. Kept apart
	/// from `TokensSaved` because the two measure different things on different
	/// denominators - content replaced versus body bytes never fetched - and one
	/// counter holding both described neither.
	pub BodyBytesSaved:AtomicU64,
	pub CCRHits:AtomicU64,
	pub CCRMisses:AtomicU64,
	pub CCRCreated:AtomicU64,
	pub ToolRelayCalls:AtomicU64,
	pub CompressionRatioEma:AtomicU64, // ×100 for EMA of compression ratio

	// LLM API response cache (request identity → compressed response)
	/// LRU cache: request identity → (inserted-at, serialized response body).
	/// The key is the whole `Cache::Identity` - its `Hash` is the digest, its `Eq`
	/// the canonical request bytes - so a hit is equality and not a digest match.
	/// F5 (report 06): entries never expired on their own - a marker minted at
	/// minute 0 of a long session was replayed unchanged at minute 90, silently
	/// diverging from what a fresh (possibly temperature>0) upstream call would
	/// return. Checked against `ResponseCacheTtl` on the hit path.
	/// std::sync::Mutex rule: a held guard makes the future non-Send, so this
	/// lock is NEVER held across an `.await`; any future code that must hold
	/// it across one switches this field to `tokio::sync::Mutex`.
	pub ResponseCache:std::sync::Mutex<lru::LruCache<CacheIdentity, CacheEntry>>,
	/// TTL applied to `ResponseCache` entries - reuses `cli.ccr_ttl_seconds`
	/// so cached LLM responses don't outlive the CCR content they were
	/// derived from by a different, uncoordinated lifetime.
	pub ResponseCacheTtl:std::time::Duration,
	pub CacheHits:AtomicU64,
	pub CacheMisses:AtomicU64,

	/// Tracks async background tasks (tool relay callbacks, CCR notifications)
	/// so shutdown waits for them to complete before exiting.
	pub TaskTracker:TaskTracker,

	/// Headroom fill percentage (×100, 0-10000). Updated after each
	/// compression. Derived from compression_ratio_ema: higher compression =
	/// lower fill = more headroom. Used by the Python plugin to set
	/// x-headroom-budget for adaptive compression.
	pub FillPct:AtomicU64,

	// Extended metrics
	pub InlineCCRHits:AtomicU64,
	pub InlineCCRMisses:AtomicU64,
	pub ToolRelaySuccess:AtomicU64,
	pub ToolRelayFailure:AtomicU64,
	pub NotifySuccess:AtomicU64,
	pub NotifyFailure:AtomicU64,
	pub UpstreamErrors4xx:AtomicU64,
	pub UpstreamErrors5xx:AtomicU64,
	pub UpstreamTimeouts:AtomicU64,
	/// Non-timeout transport failures (connection refused, DNS, TLS) - split
	/// from `UpstreamTimeouts` (F17), which previously counted every kind
	/// of transport error as a "timeout".
	pub UpstreamConnectErrors:AtomicU64,
	/// Transport failures that are neither a timeout nor a connect error -
	/// a body error, a malformed URL, a TLS handshake that got past connect.
	/// Split out of `UpstreamConnectErrors`, which used to absorb all of them,
	/// so `connect_errors` means what it says.
	pub UpstreamOtherErrors:AtomicU64,
	/// 02-F9: mid-stream chunk errors on the SSE relay path - distinct from
	/// `UpstreamConnectErrors`/`UpstreamTimeouts`, which only ever see
	/// failures before the response status/headers arrive. Once
	/// `Body::from_stream` owns the stream, a later chunk `Err` (the F2
	/// failure mode: an upstream that hangs or drops mid-stream) was
	/// otherwise invisible to every counter - a 200 status had already been
	/// recorded, so an operator diagnosing "streams cut off" from `/stats`
	/// saw nothing.
	pub SSEStreamErrors:AtomicU64,
	pub RequestBodyBytes:AtomicU64,
	pub ResponseBodyBytes:AtomicU64,
	pub UpstreamLatencyMicros:AtomicU64,

	/// TTL cache for the `/health/upstream` probe result: `(ok, checked_at)`.
	/// F19: without this, a monitor polling `/health/upstream` every 10-15s
	/// re-probes the real upstream on every single call - the exact cost
	/// class `Maintain/examples/08_health_upstream.py` documents (a live
	/// upstream call was already removed from the plain `/health` endpoint
	/// for this reason; `/health/upstream` just re-introduced it under a
	/// different path).
	pub UpstreamHealthCache:std::sync::Mutex<Option<(bool, std::time::Instant)>>,

	/// Live compression thresholds (report 07 F2/F4/T15) - previously
	/// `CacheCompressThreshold`/`TokenCompressThreshold`/
	/// `InlineCCRThreshold` were consts, so every shipped TOML's
	/// `[compression]` table was parsed and then silently discarded; `POST
	/// /reload` and the config-file watcher logged success while applying
	/// nothing. Resolved once at startup (env > TOML > const default, see
	/// `ResolveThresholds`) and updated in place by both the watcher and
	/// `/reload` - this is what makes hot-reload real instead of theater.
	pub CacheCompressThreshold:AtomicUsize,
	pub TokenCompressThreshold:AtomicUsize,
	pub InlineCCRThreshold:AtomicUsize,
	/// `CodeMultiplier`, ×100 for integer atomic storage (matches the
	/// existing `CompressionRatioEma` ×100 convention above).
	pub CodeMultiplierX100:AtomicU64,
	/// `[compression] auto_expand` (pre-Rust port): when true, the cache and
	/// token compress paths resolve the just-stored content inline instead
	/// of emitting a `<<<CCR:...>>>` marker, capped at `AutoExpandLimit`
	/// bytes (content larger than the limit still emits the marker). Same
	/// live-tunable discipline as the thresholds above - resolved at startup
	/// (env > TOML > false, see `ResolveThresholds`) and re-applied by
	/// `POST /reload`. Default false = byte-identical current behavior
	/// (markers appear).
	pub AutoExpand:AtomicBool,
	/// `[compression] auto_expand_limit` - byte cap for auto-expanded
	/// content (see `AutoExpand`). Default `AutoExpandLimitDefault`
	/// (matches the Hermes-side `AphroditeState` default).
	pub AutoExpandLimit:AtomicUsize,
	/// `APHRODITE_MAX_REQUEST_BODY_SIZE` (pre-Rust port): request-body cap
	/// in bytes enforced by the [`CappedBody`] extractor while the body is
	/// read - `Some(n)` rejects over-`n` bodies with 413, `None` (the
	/// default) = no cap, byte-identical current behavior. Matches the
	/// Hermes-side `AphroditeState::max_request_body_size` (same env/TOML
	/// key, resolved by `ResolveThresholds`). Write-once: set in
	/// `build_state`, never mutated after startup.
	pub MaximumRequestBodySize:Option<usize>,
}
