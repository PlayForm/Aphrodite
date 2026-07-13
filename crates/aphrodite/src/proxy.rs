//! aphrodite - Reverse proxy with Chat Completions API support.
//!
//! Two modes:
//! - **Cache** (:9797): In-memory CCR, lightweight compression, no tool
//!   injection. Passes most content through, only compresses very large outputs
//!   (>8KB).
//! - **Token** (:9798): SQLite CCR, aggressive compression, tool injection,
//!   tool relay for bidirectional Hermes communication.
//!
//! Chat Completions API:
//! - Forwards POST /v1/chat/completions to DeepSeek
//! - Intercepts responses, compresses tool output via CCR
//! - Does NOT inject the aphrodite_retrieve tool definition into response
//!   tool_calls (that was tried and reverted - see Bug 18 in
//!   `compress_chat_completion`); the Python plugin registers the tool
//!   instead.

use std::{
	collections::{HashMap, VecDeque},
	num::NonZeroUsize,
	sync::{
		Arc,
		Mutex,
		atomic::{AtomicU64, AtomicUsize, Ordering},
	},
	time::Duration,
};

use axum::{
	body::Body,
	extract::State,
	http::{Method, StatusCode},
	response::{IntoResponse, Json, Response},
};
use bytes::Bytes;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use headroom_core::ccr::{
	CcrStore,
	backends::{in_memory::InMemoryCcrStore, sqlite::SqliteCcrStore},
	compute_key,
};
use tokio_util::task::TaskTracker;
use futures::StreamExt;

/// API key wrapper with safe Debug and Display - never leaks to logs.
#[derive(Clone)]
pub struct Secret(pub(crate) String);

impl std::fmt::Debug for Secret {
	fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "[REDACTED]") }
}

impl std::fmt::Display for Secret {
	fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "[REDACTED]") }
}

impl Secret {
	/// Expose the raw API key value for use in HTTP headers.
	pub fn expose(&self) -> &str { &self.0 }
}

impl From<&str> for Secret {
	fn from(s:&str) -> Self { Secret(s.to_string()) }
}

impl From<String> for Secret {
	fn from(s:String) -> Self { Secret(s) }
}

use crate::config::{Cli, CompressionConfig, ProxyMode, env_parse_warn};

// ── Constants ───────────────────────────────────────────────────────

/// Content size threshold for cache mode compression (8KB).
const CACHE_COMPRESS_THRESHOLD:usize = 8192;
/// Content size threshold for aphrodite mode compression (1KB).
const TOKEN_COMPRESS_THRESHOLD:usize = 1024;
/// Inline CCR threshold - content below this size is stored in the inline
/// HashMap instead of SQLite/in-memory CCR backends, avoiding round-trip
/// overhead.
const INLINE_CCR_THRESHOLD:usize = 256;
/// Chat Completions API path.
const CHAT_COMPLETIONS_PATH:&str = "/v1/chat/completions";
/// Max body size cached in `response_cache` (1 MB). `response_cache` is
/// count-bounded (128 entries via LRU) but not byte-bounded (report 06 F7) -
/// without this, 128 large completions (e.g. big code-generation responses)
/// could hold well over 100 MB resident for a cache whose only purpose is
/// avoiding a repeat upstream round-trip on an identical request.
const RESPONSE_CACHE_MAX_BODY_BYTES:usize = 1024 * 1024;

/// Non-streaming response body cap (report 02-T10). A single buffered
/// `.bytes().await` can exhaust memory if the upstream returns a huge body
/// (e.g. a full codebase in a single completion). Streamed (SSE) responses
/// bypass this limit entirely — they're chunked at the protocol level.
const RESPONSE_MAX_BODY_BYTES:usize = 64 * 1024 * 1024; // 64 MB

/// Live-resolved compression thresholds (report 07 F2/F4/T15): env var >
/// TOML `[compression]` value > compiled-in default, the same precedence
/// pattern `apply_port_override` already uses for the listen port. Computed
/// once at startup (`build_state`) and re-computed on every hot-reload
/// (config-file watcher + `POST /reload`) so both actually change the live
/// proxy instead of only re-parsing and logging.
pub struct ResolvedThresholds {
	pub cache:usize,
	pub token:usize,
	pub inline:usize,
	pub code_multiplier:f64,
}

/// Resolve the four compression thresholds from env vars, the TOML
/// `[compression]` table (if any), and the compiled-in defaults, in that
/// precedence order.
///
/// `code_multiplier` defaults to `3.0`, not the old free-function's `2`: the
/// prior default only existed because `APHRODITE_CODE_MULTIPLIER=3.0`
/// silently failed a `usize` parse (report 07 F10) - every shipped TOML and
/// doc has always said `3.0`, so `2` was a masked bug, not an intentional
/// value, and this restores the value the project's own config always
/// claimed.
pub fn resolve_thresholds(compression:Option<&CompressionConfig>) -> ResolvedThresholds {
	ResolvedThresholds {
		cache:env_parse_warn::<usize>("APHRODITE_TOOL_THRESHOLD_CACHE")
			.or_else(|| compression.and_then(|c| c.tool_threshold_cache).map(|v| v as usize))
			.unwrap_or(CACHE_COMPRESS_THRESHOLD),
		token:env_parse_warn::<usize>("APHRODITE_TOOL_THRESHOLD_TOKEN")
			.or_else(|| compression.and_then(|c| c.tool_threshold_token).map(|v| v as usize))
			.unwrap_or(TOKEN_COMPRESS_THRESHOLD),
		inline:env_parse_warn::<usize>("APHRODITE_INLINE_THRESHOLD")
			.or_else(|| compression.and_then(|c| c.inline_threshold).map(|v| v as usize))
			.unwrap_or(INLINE_CCR_THRESHOLD),
		code_multiplier:env_parse_warn::<f64>("APHRODITE_CODE_MULTIPLIER")
			.or_else(|| compression.and_then(|c| c.code_multiplier))
			.unwrap_or(3.0),
	}
}

// ── spawn_blocking wrappers for CcrStore (rusqlite is blocking) ─────

/// Wrapper for `ccr.get()` on a blocking thread.
pub(crate) async fn ccr_get(ccr:&Arc<dyn CcrStore>, hash:&str) -> Option<String> {
	let ccr = ccr.clone();
	let hash = hash.to_owned();
	tokio::task::spawn_blocking(move || ccr.get(&hash)).await.unwrap_or(None)
}

/// Wrapper for `ccr.put()` on a blocking thread.
/// Store `content` under `hash` in the CCR backend. Returns whether the
/// store actually succeeded (F4) - a caller that discards this and replaces
/// the original content with a marker anyway ships a marker whose hash
/// resolves to nothing the moment the store is full/locked/panics, which is
/// permanent data loss (the original content never reached the client).
async fn ccr_put(ccr:&Arc<dyn CcrStore>, hash:&str, content:&str) -> bool {
	let ccr = ccr.clone();
	let hash = hash.to_owned();
	let content = content.to_owned();
	tokio::task::spawn_blocking(move || ccr.put(&hash, &content))
		.await
		.unwrap_or(false)
}

/// Wrapper for `ccr.del()` on a blocking thread.
async fn ccr_del(ccr:&Arc<dyn CcrStore>, hash:&str) -> bool {
	let ccr = ccr.clone();
	let hash = hash.to_owned();
	tokio::task::spawn_blocking(move || ccr.del(&hash)).await.unwrap_or(false)
}

/// Wrapper for `ccr.len()` on a blocking thread.
async fn ccr_len(ccr:&Arc<dyn CcrStore>) -> usize {
	let ccr = ccr.clone();
	tokio::task::spawn_blocking(move || ccr.len()).await.unwrap_or(0)
}

// ── State ──────────────────────────────────────────────────────────

/// Shared proxy state: upstream client config, CCR backend, and all
/// counters/caches used by request handlers. Wrapped in `Arc` and cloned
/// into every axum handler.
pub struct AppState {
	pub client:HttpClient,
	pub api_url:String,
	pub model:String,
	pub api_key:Secret,
	pub ccr:Option<Arc<dyn CcrStore>>,
	pub add_markers:bool,
	pub mode:ProxyMode,
	pub tool_relay:bool,
	pub notify_url:Option<String>,
	pub notify_key:Option<String>,
	/// Dev mode - verbose logging.
	pub dev:bool,

	// Structured debug
	/// Ring buffer of last 50 request summaries
	/// Lock uses `.lock().map(...).unwrap_or_default()` - poison is safely
	/// tolerated: a poisoned mutex returns Err, and unwrap_or_default gives
	/// an empty/logical-default so the proxy stays up.
	pub request_history:std::sync::Mutex<VecDeque<serde_json::Value>>,
	/// Inline CCR for tiny entries - no round-trip needed (< INLINE_CCR_THRESHOLD
	/// bytes). Lock uses `.lock().map(...)` - same poison safety pattern.
	/// Bounded to 1024 entries via LruCache to prevent unbounded memory growth.
	pub inline_ccr:std::sync::Mutex<lru::LruCache<String, String>>,

	// Stats
	/// Latency histogram buckets (microseconds): 1ms, 10ms, 100ms, 1s, 10s
	pub latency_buckets:[AtomicU64; 5],
	/// Running total latency in microseconds for Prometheus _sum
	pub total_latency_micros:AtomicU64,
	/// Track last N errors for hot-path analysis
	/// Mapped through `.lock().map(...)` - a poisoned lock returns Err and
	/// unwrap_or_default provides an empty Vec so error recording degrades
	/// gracefully without crashing the proxy.
	pub last_errors:std::sync::Mutex<VecDeque<String>>,
	/// Compression decision counters by content type
	/// Uses `.lock().map(...)` - poison tolerant by design.
	pub compressions_by_type:std::sync::Mutex<std::collections::HashMap<String, u64>>,

	// Stats
	pub requests_total:AtomicU64,
	pub requests_compressed:AtomicU64,
	/// Cumulative bytes saved by compression/caching, despite the name
	/// (report 05 F5: the field is exposed as `tokens_saved` in `/stats` and
	/// that external API name is kept for compatibility, but every call site
	/// now accumulates raw BYTES - originally-removed content length minus
	/// whatever replaced it (a rendered marker, or nothing at all on a full
	/// cache hit) - never a token estimate. Previously one site divided by 4
	/// to estimate tokens while every other site counted bytes, making the
	/// counter internally inconsistent by 4x; another subtracted the bare
	/// 40-char hash length instead of the actual (much longer) marker length,
	/// overstating savings.
	pub tokens_saved:AtomicU64,
	pub ccr_hits:AtomicU64,
	pub ccr_misses:AtomicU64,
	pub ccr_created:AtomicU64,
	pub tool_relay_calls:AtomicU64,
	pub compression_ratio_ema:AtomicU64, // ×100 for EMA of compression ratio

	// LLM API response cache (model+messages → compressed response)
	/// LRU cache: hash(model+messages) → (inserted-at, serialized response body).
	/// F5 (report 06): entries never expired on their own - a marker minted at
	/// minute 0 of a long session was replayed unchanged at minute 90, silently
	/// diverging from what a fresh (possibly temperature>0) upstream call would
	/// return. Checked against `response_cache_ttl` on the hit path.
	pub response_cache:std::sync::Mutex<lru::LruCache<u64, (std::time::Instant, Vec<u8>)>>,
	/// TTL applied to `response_cache` entries - reuses `cli.ccr_ttl_seconds`
	/// so cached LLM responses don't outlive the CCR content they were
	/// derived from by a different, uncoordinated lifetime.
	pub response_cache_ttl:std::time::Duration,
	pub cache_hits:AtomicU64,
	pub cache_misses:AtomicU64,

	/// Tracks async background tasks (tool relay callbacks, CCR notifications)
	/// so shutdown waits for them to complete before exiting.
	pub task_tracker:TaskTracker,

	/// Headroom fill percentage (×100, 0-10000). Updated after each
	/// compression. Derived from compression_ratio_ema: higher compression =
	/// lower fill = more headroom. Used by the Python plugin to set
	/// x-headroom-budget for adaptive compression.
	pub fill_pct:AtomicU64,

	// Extended metrics
	pub inline_ccr_hits:AtomicU64,
	pub inline_ccr_misses:AtomicU64,
	pub tool_relay_success:AtomicU64,
	pub tool_relay_failure:AtomicU64,
	pub notify_success:AtomicU64,
	pub notify_failure:AtomicU64,
	pub upstream_errors_4xx:AtomicU64,
	pub upstream_errors_5xx:AtomicU64,
	pub upstream_timeouts:AtomicU64,
	/// Non-timeout transport failures (connection refused, DNS, TLS) - split
	/// from `upstream_timeouts` (F17), which previously counted every kind
	/// of transport error as a "timeout".
	pub upstream_connect_errors:AtomicU64,
	pub ccr_store_entries:AtomicU64,
	pub ccr_store_bytes:AtomicU64,
	pub request_body_bytes:AtomicU64,
	pub response_body_bytes:AtomicU64,
	pub upstream_latency_micros:AtomicU64,

	/// TTL cache for the `/health/upstream` probe result: `(ok, checked_at)`.
	/// F19: without this, a monitor polling `/health/upstream` every 10-15s
	/// re-probes the real upstream on every single call - the exact cost
	/// class `Maintain/examples/08_health_upstream.py` documents (a live
	/// upstream call was already removed from the plain `/health` endpoint
	/// for this reason; `/health/upstream` just re-introduced it under a
	/// different path).
	pub upstream_health_cache:std::sync::Mutex<Option<(bool, std::time::Instant)>>,

	/// Live compression thresholds (report 07 F2/F4/T15) - previously
	/// `CACHE_COMPRESS_THRESHOLD`/`TOKEN_COMPRESS_THRESHOLD`/
	/// `INLINE_CCR_THRESHOLD` were consts, so every shipped TOML's
	/// `[compression]` table was parsed and then silently discarded; `POST
	/// /reload` and the config-file watcher logged success while applying
	/// nothing. Resolved once at startup (env > TOML > const default, see
	/// `resolve_thresholds`) and updated in place by both the watcher and
	/// `/reload` - this is what makes hot-reload real instead of theater.
	pub cache_compress_threshold:AtomicUsize,
	pub token_compress_threshold:AtomicUsize,
	pub inline_ccr_threshold:AtomicUsize,
	/// `code_multiplier`, ×100 for integer atomic storage (matches the
	/// existing `compression_ratio_ema` ×100 convention above).
	pub code_multiplier_x100:AtomicU64,
}

impl AppState {
	pub fn stats_json(&self) -> serde_json::Value {
		serde_json::json!({
			"mode": match self.mode {
				ProxyMode::Cache => "cache",
				ProxyMode::Token => "token",
			},
			"proxy": "aphrodite",
			"ccr_backend": if self.ccr.is_some() { "enabled" } else { "none" },
			// Renamed from "tool_relay" (F12): a duplicate "tool_relay" key
			// further down (the stats object) silently won in
			// `serde_json::json!`'s map construction, so this boolean -
			// whether tool relay is enabled at all - was never actually
			// exposed; there was no way to distinguish "relay disabled"
			// from "relay enabled, zero calls".
			"tool_relay_enabled": self.tool_relay,
			"requests": {
				"total": self.requests_total.load(Ordering::Relaxed),
				"compressed": self.requests_compressed.load(Ordering::Relaxed),
			},
			"tokens_saved": self.tokens_saved.load(Ordering::Relaxed),
			"ccr": {
				"hits": self.ccr_hits.load(Ordering::Relaxed),
				"misses": self.ccr_misses.load(Ordering::Relaxed),
				"created": self.ccr_created.load(Ordering::Relaxed),
			},
			"tool_relay_calls": self.tool_relay_calls.load(Ordering::Relaxed),
			"cache": {
				"hits": self.cache_hits.load(Ordering::Relaxed),
				"misses": self.cache_misses.load(Ordering::Relaxed),
			},
			"latency_buckets_us": [
				self.latency_buckets[0].load(Ordering::Relaxed),
				self.latency_buckets[1].load(Ordering::Relaxed),
				self.latency_buckets[2].load(Ordering::Relaxed),
				self.latency_buckets[3].load(Ordering::Relaxed),
				self.latency_buckets[4].load(Ordering::Relaxed),
			],
			"total_latency_micros": self.total_latency_micros.load(Ordering::Relaxed),
			"compressions_by_type": self.compressions_by_type.lock().map(|m| m.clone()).unwrap_or_default(),
			"compression_ratio_ema": self.compression_ratio_ema.load(Ordering::Relaxed) as f64 / 100.0,
			"last_errors": self.last_errors.lock().map(|v| v.iter().rev().take(5).cloned().collect::<Vec<_>>()).unwrap_or_default(),
			"request_history": self.request_history.lock().map(|v| v.clone()).unwrap_or_default(),
			"inline_ccr": {
				"hits": self.inline_ccr_hits.load(Ordering::Relaxed),
				"misses": self.inline_ccr_misses.load(Ordering::Relaxed),
			},
			"tool_relay": {
				"total": self.tool_relay_calls.load(Ordering::Relaxed),
				"success": self.tool_relay_success.load(Ordering::Relaxed),
				"failure": self.tool_relay_failure.load(Ordering::Relaxed),
			},
			"notify": {
				"success": self.notify_success.load(Ordering::Relaxed),
				"failure": self.notify_failure.load(Ordering::Relaxed),
			},
			"upstream_errors": {
				"4xx": self.upstream_errors_4xx.load(Ordering::Relaxed),
				"5xx": self.upstream_errors_5xx.load(Ordering::Relaxed),
				"timeouts": self.upstream_timeouts.load(Ordering::Relaxed),
				// F17: non-timeout transport failures (connect refused, DNS,
				// TLS) - previously folded into "timeouts" above.
				"connect_errors": self.upstream_connect_errors.load(Ordering::Relaxed),
			},
			"ccr_store": {
				"entries": self.ccr_store_entries.load(Ordering::Relaxed),
				"bytes_approx": self.ccr_store_bytes.load(Ordering::Relaxed),
			},
			"body_bytes": {
				"request": self.request_body_bytes.load(Ordering::Relaxed),
				"response": self.response_body_bytes.load(Ordering::Relaxed),
			},
			"upstream_latency_micros": self.upstream_latency_micros.load(Ordering::Relaxed),
		})
	}

	fn compress_threshold(&self) -> usize {
		match self.mode {
			ProxyMode::Cache => self.cache_compress_threshold.load(Ordering::Relaxed),
			ProxyMode::Token => self.token_compress_threshold.load(Ordering::Relaxed),
		}
	}

	/// Inline-vs-durable storage cutoff (report 07 F2/T15) - was the
	/// `INLINE_CCR_THRESHOLD` const; now live-configurable via
	/// `compression.inline_threshold`.
	fn inline_ccr_threshold(&self) -> usize { self.inline_ccr_threshold.load(Ordering::Relaxed) }

	/// How many times the base threshold for code content (report 07
	/// F2/F10/T11/T15) - was a free fn parsing `APHRODITE_CODE_MULTIPLIER` as
	/// `usize` (silently truncating the documented `3.0` to a parse failure
	/// -> default 2); now live-configurable via `compression.code_multiplier`
	/// and re-resolved as `f64` on every hot-reload.
	fn code_multiplier(&self) -> f64 { self.code_multiplier_x100.load(Ordering::Relaxed) as f64 / 100.0 }

	/// Per-type threshold - code stays in context longer, logs compressed
	/// aggressively.
	fn threshold_for(&self, ct:&str) -> usize {
		let base = self.compress_threshold();
		// Noisy types: keep at base threshold - coding sessions need build output
		// visible
		match ct {
			"linter" | "build_output" | "log" => return base,
			_ => {},
		}
		// Auto-tune: adjust thresholds based on historical compression ratios
		let ratio = self.compression_ratio_ema.load(Ordering::Relaxed) as f64 / 100.0;
		let tune = if ratio > 20.0 {
			// Very aggressive - raise thresholds to preserve more content
			2.0
		} else if ratio < 3.0 && ratio > 0.0 {
			// Very conservative - lower thresholds to compress more
			0.5
		} else {
			1.0
		};
		let base = (base as f64 * tune) as usize;
		match ct {
			"error" => base * 8,
			"code_rust" | "code_python" | "code_go" | "code_js" | "code" => (base as f64 * self.code_multiplier()) as usize,
			"diff" | "git" => base * 2,
			"text" => base * 2,
			"tool_output" => base,
			"json" => base,
			_ => base,
		}
	}

	fn update_compression_ratio(&self, original_len:usize, compressed_len:usize) {
		if original_len == 0 || compressed_len == 0 {
			return;
		}
		let ratio = (original_len as f64 / compressed_len as f64 * 100.0) as u64;
		// Exponential moving average: new = 0.2 * ratio + 0.8 * old
		let old = self.compression_ratio_ema.load(Ordering::Relaxed);
		let new = ((ratio as f64 * 0.2) + (old as f64 * 0.8)) as u64;
		self.compression_ratio_ema.store(new, Ordering::Relaxed);
		// After each compression update, also update fill_pct for headroom feedback
		// loop
		self.compute_fill_pct();
	}

	/// Derive fill percentage from compression ratio EMA.
	/// Higher compression ratio → lower fill → more headroom.
	/// fill_pct = 100 - (ratio_ema / 20), clamped to [1..99].
	fn compute_fill_pct(&self) {
		let ratio_ema = self.compression_ratio_ema.load(Ordering::Relaxed);
		let pct = if ratio_ema == 0 {
			99u64
		} else {
			let raw = 100u64.saturating_sub(ratio_ema / 20);
			raw.clamp(1, 99)
		};
		self.fill_pct.store(pct * 100, Ordering::Relaxed); // ×100 for precision
	}

	fn record_latency(&self, d:std::time::Duration) {
		let us = d.as_micros() as u64;
		let bucket = if us < 1_000 {
			0
		} else if us < 10_000 {
			1
		} else if us < 100_000 {
			2
		} else if us < 1_000_000 {
			3
		} else {
			4
		};
		self.latency_buckets[bucket].fetch_add(1, Ordering::Relaxed);
		self.total_latency_micros.fetch_add(us, Ordering::Relaxed);
	}

	fn record_error(&self, msg:String) {
		if let Ok(mut v) = self.last_errors.lock() {
			v.push_back(msg);
			if v.len() > 100 {
				v.pop_front();
			}
		}
	}

	fn record_compression(&self, ct:&str) {
		if let Ok(mut m) = self.compressions_by_type.lock() {
			*m.entry(ct.to_string()).or_insert(0) += 1;
		}
	}

	fn record_request(&self, id:&str, method:&str, path:&str, status:u16, compressed:bool, elapsed_ms:u128) {
		if let Ok(mut hist) = self.request_history.lock() {
			hist.push_back(serde_json::json!({
				"id": id,
				"method": method,
				"path": path,
				"status": status,
				"compressed": compressed,
				"elapsed_ms": elapsed_ms,
			}));
			if hist.len() > 50 {
				hist.pop_front();
			}
		}
	}
}

// ── Tool relay types ────────────────────────────────────────────────

/// Inbound request body for `POST /tool_relay`: a Hermes-side tool call to
/// execute against this proxy's CCR state (e.g. `aphrodite_retrieve`).
#[derive(Debug, Deserialize)]
pub struct ToolRelayRequest {
	pub tool:String,
	pub params:serde_json::Value,
	pub callback_url:Option<String>,
}

/// Response for a tool relay call. When `callback_url` was set on the
/// request, the call runs asynchronously and this comes back immediately
/// with `async_call:true` and no `result` - the real result is POSTed to
/// the callback URL later.
#[derive(Debug, Serialize)]
pub struct ToolRelayResponse {
	pub success:bool,
	pub result:Option<serde_json::Value>,
	pub error:Option<String>,
	pub async_call:bool,
}

// ── CCR management types ────────────────────────────────────────────

/// Inbound request body for `POST /ccr/create`: store `content` under an
/// optional caller-supplied `key` (defaults to the content hash).
#[derive(Debug, Deserialize)]
pub struct CcrCreateRequest {
	pub content:String,
	pub key:Option<String>,
	pub ttl_seconds:Option<u64>,
	pub tags:Option<Vec<String>>,
}

/// Response for `POST /ccr/create`, reporting the resulting hash and the
/// size reduction achieved.
#[derive(Debug, Serialize)]
pub struct CcrCreateResponse {
	pub hash:String,
	pub token_savings_ratio:f64,
	pub original_size:usize,
	pub compressed_size:usize,
	pub marker_size:usize,
}

/// Webhook payload POSTed to `notify_url` when a new CCR entry is created,
/// so external subscribers (e.g. Hermes) can track store growth without
/// polling.
#[derive(Debug, Serialize)]
pub struct CcrNotification {
	pub event:String,
	pub hash:String,
	pub created_at:u64,
	pub ttl:u64,
	pub tags:Vec<String>,
}

// ── Build state ─────────────────────────────────────────────────────

/// Construct `AppState` from CLI config: builds the tuned HTTP client,
/// opens the CCR backend appropriate for `cli.mode` (SQLite for Token,
/// in-memory for Cache), and zeroes all counters.
pub async fn build_state(cli:&Cli, compression:Option<&CompressionConfig>) -> anyhow::Result<AppState> {
	// Tuned HttpClient for high-concurrency API proxy workload.
	// Default pool: 100 idle connections per host, 90s idle timeout, keepalive.
	let client = HttpClient::builder()
		.timeout(std::time::Duration::from_secs(cli.timeout))
		.connect_timeout(std::time::Duration::from_secs(10))
		.pool_max_idle_per_host(100)
		.pool_idle_timeout(std::time::Duration::from_secs(90))
		.tcp_keepalive(std::time::Duration::from_secs(60))
		.build()?;

	let ccr:Option<Arc<dyn CcrStore>> = match cli.mode {
		ProxyMode::Token if !cli.no_ccr_marker => {
			let db_path = cli.ccr_db_path.as_ref().map_or_else(
				|| {
					dirs::home_dir()
						.unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
						.join(".hermes")
						.join("aphrodite")
						.join("ccr.db")
				},
				|p| p.clone(),
			);
			// Ensure parent directories exist before opening SQLite DB.
			// Without this, a missing ~/.hermes/aphrodite/ directory causes
			// the token proxy to fail silently at startup while the cache
			// proxy continues running - a partial-failure that's invisible
			// to the plugin because stderr is piped to DEVNULL.
			if let Some(parent) = db_path.parent() {
				std::fs::create_dir_all(parent)
					.map_err(|e| anyhow::anyhow!("SQLite CCR: cannot create directory {}: {}", parent.display(), e))?;
			}
			let store = SqliteCcrStore::open(&db_path, cli.ccr_ttl_seconds)
				.map_err(|e| anyhow::anyhow!("SQLite CCR: {}", e))?;
			Some(Arc::new(store))
		},
		ProxyMode::Cache => {
			let store =
				InMemoryCcrStore::with_capacity_and_ttl(10_000, std::time::Duration::from_secs(cli.ccr_ttl_seconds));
			Some(Arc::new(store))
		},
		_ => None,
	};

	let thresholds = resolve_thresholds(compression);

	Ok(AppState {
		client,
		api_url:cli.api_url.clone(),
		model:cli.model.clone(),
		api_key:cli.api_key.clone().into(),
		ccr,
		add_markers:!cli.no_ccr_marker,
		mode:cli.mode,
		tool_relay:cli.tool_relay,
		notify_url:cli.notify_url.clone(),
		notify_key:cli.notify_key.clone(),
		dev:cli.dev,
		latency_buckets:[
			AtomicU64::new(0),
			AtomicU64::new(0),
			AtomicU64::new(0),
			AtomicU64::new(0),
			AtomicU64::new(0),
		],
		total_latency_micros:AtomicU64::new(0),
		last_errors:Mutex::new(VecDeque::new()),
		compressions_by_type:Mutex::new(HashMap::new()),
		request_history:Mutex::new(VecDeque::new()),
		inline_ccr:Mutex::new(lru::LruCache::new(NonZeroUsize::new(1024).unwrap())),
		requests_total:AtomicU64::new(0),
		requests_compressed:AtomicU64::new(0),
		tokens_saved:AtomicU64::new(0),
		ccr_hits:AtomicU64::new(0),
		ccr_misses:AtomicU64::new(0),
		ccr_created:AtomicU64::new(0),
		tool_relay_calls:AtomicU64::new(0),
		compression_ratio_ema:AtomicU64::new(200), // initial: 2.0x - conservative, avoids startup scale-up
		response_cache:Mutex::new(lru::LruCache::new(NonZeroUsize::new(128).unwrap())),
		response_cache_ttl:std::time::Duration::from_secs(cli.ccr_ttl_seconds),
		cache_hits:AtomicU64::new(0),
		cache_misses:AtomicU64::new(0),
		fill_pct:AtomicU64::new(9000), // 90.00% - moderate fill initial default
		task_tracker:TaskTracker::new(),

		inline_ccr_hits:AtomicU64::new(0),
		inline_ccr_misses:AtomicU64::new(0),
		tool_relay_success:AtomicU64::new(0),
		tool_relay_failure:AtomicU64::new(0),
		notify_success:AtomicU64::new(0),
		notify_failure:AtomicU64::new(0),
		upstream_errors_4xx:AtomicU64::new(0),
		upstream_errors_5xx:AtomicU64::new(0),
		upstream_timeouts:AtomicU64::new(0),
		upstream_connect_errors:AtomicU64::new(0),
		ccr_store_entries:AtomicU64::new(0),
		ccr_store_bytes:AtomicU64::new(0),
		request_body_bytes:AtomicU64::new(0),
		response_body_bytes:AtomicU64::new(0),
		upstream_latency_micros:AtomicU64::new(0),
		upstream_health_cache:std::sync::Mutex::new(None),
		cache_compress_threshold:AtomicUsize::new(thresholds.cache),
		token_compress_threshold:AtomicUsize::new(thresholds.token),
		inline_ccr_threshold:AtomicUsize::new(thresholds.inline),
		code_multiplier_x100:AtomicU64::new((thresholds.code_multiplier * 100.0) as u64),
	})
}

// ── Main proxy handler ──────────────────────────────────────────────

/// Compute a cache key from a Chat Completions request body: hash(api_key +
/// model + messages). Uses FNV-1a (deterministic across restarts, unlike
/// DefaultHasher). Includes api_key to prevent cross-user cache collision.
/// Returns None if the body can't be parsed as JSON or lacks model/messages.
fn cache_key_from_body(body:&[u8], api_key:&str) -> Option<u64> {
	let v:serde_json::Value = serde_json::from_slice(body).ok()?;
	// F3: never cache a streamed request - the cached entry is a single
	// buffered JSON body, replayed with `Content-Type: application/json`,
	// which is nothing like an SSE stream a `"stream": true` client expects.
	if v.get("stream").and_then(|s| s.as_bool()).unwrap_or(false) {
		return None;
	}
	// `model`/`messages` must both be present for this to be a valid,
	// cacheable chat-completion request.
	v.get("model")?.as_str()?;
	v.get("messages")?;
	// F3: the key used to hash only api_key+model+messages, so two requests
	// differing solely in `tools`, `tool_choice`, `temperature`, `top_p`,
	// `n`, or `response_format` collided and got served each other's cached
	// response. Include every field that changes what a valid response can
	// look like, in a fixed, canonical order (serde_json's key order from
	// `v` itself is not guaranteed stable across equivalent requests).
	let mut parts:Vec<u8> = Vec::new();
	parts.extend_from_slice(api_key.as_bytes());
	for (label, val) in [
		("model", v.get("model")),
		("messages", v.get("messages")),
		("tools", v.get("tools")),
		("tool_choice", v.get("tool_choice")),
		("temperature", v.get("temperature")),
		("top_p", v.get("top_p")),
		("n", v.get("n")),
		("response_format", v.get("response_format")),
	] {
		parts.push(b':');
		parts.extend_from_slice(label.as_bytes());
		parts.push(b'=');
		if let Some(val) = val {
			parts.extend_from_slice(serde_json::to_string(val).ok()?.as_bytes());
		}
	}
	// FNV-1a 64-bit hash - deterministic across process restarts
	Some(fnv1a_64(&parts))
}

/// Look up `ck` in `state.response_cache`, treating an entry older than
/// `state.response_cache_ttl` as a miss and evicting it (report 06 F5) -
/// without this, a marker minted at minute 0 of a long session gets its
/// cached response replayed unchanged at minute 90, silently diverging from
/// what a fresh (possibly temperature>0) upstream call would return.
fn response_cache_get(state:&AppState, ck:u64) -> Option<Vec<u8>> {
	state.response_cache.lock().ok().and_then(|mut cache| {
		let expired = cache
			.peek(&ck)
			.map(|(inserted_at, _)| inserted_at.elapsed() >= state.response_cache_ttl)
			.unwrap_or(false);
		if expired {
			cache.pop(&ck);
			None
		} else {
			cache.get(&ck).map(|(_, body)| body.clone())
		}
	})
}

/// Copy upstream response headers onto `builder`, skipping hop-by-hop
/// headers (F5) - `content-length` is always skipped too since the caller
/// may be sending a re-serialized body of a different length than upstream's,
/// and `content-type`/the two `X-Aphrodite-*` headers the caller sets itself
/// afterward win if there's a name collision (axum keeps both; callers add
/// their own explicit `content-type` after calling this).
fn copy_upstream_headers(
	mut builder:axum::http::response::Builder,
	upstream_headers:&reqwest::header::HeaderMap,
) -> axum::http::response::Builder {
	const SKIP:&[&str] = &[
		"content-length",
		"content-type",
		"transfer-encoding",
		"connection",
		"keep-alive",
	];
	for (name, value) in upstream_headers.iter() {
		if SKIP.contains(&name.as_str()) {
			continue;
		}
		if let Ok(v) = axum::http::HeaderValue::from_bytes(value.as_bytes()) {
			builder = builder.header(name.as_str(), v);
		}
	}
	builder
}

/// Accumulate the full response body with a byte cap (report 02-T10).
/// Replaces the single unbounded `response.bytes().await` in the
/// non-streaming branch — returns a 502 error if the upstream exceeds
/// `max_bytes`, protecting the proxy's memory from a single huge response.
async fn accumulate_body(
	response: reqwest::Response,
	max_bytes: usize,
) -> Result<bytes::Bytes, String> {
	let mut buf = Vec::new();
	let mut stream = response.bytes_stream();
	while let Some(chunk) = stream.next().await {
		match chunk {
			Ok(b) => {
				if buf.len() + b.len() > max_bytes {
					return Err(format!(
						"response body exceeded {} MB limit",
						max_bytes / (1024 * 1024)
					));
				}
				buf.extend_from_slice(&b);
			},
			Err(e) => return Err(format!("body read: {}", e)),
		}
	}
	Ok(bytes::Bytes::from(buf))
}

/// FNV-1a 64-bit hash over bytes. Deterministic across restarts.
fn fnv1a_64(bytes:&[u8]) -> u64 {
	const FNV_OFFSET:u64 = 14695981039346656037;
	const FNV_PRIME:u64 = 1099511628211;
	let mut hash = FNV_OFFSET;
	for &b in bytes {
		hash ^= b as u64;
		hash = hash.wrapping_mul(FNV_PRIME);
	}
	hash
}

/// Catch-all proxy handler - forwards any request to DeepSeek.
/// Specifically handles Chat Completions API at /v1/chat/completions.
pub async fn proxy_handler(
	State(state):State<Arc<AppState>>,
	method:Method,
	path:axum::extract::OriginalUri,
	headers:axum::http::HeaderMap,
	body:Bytes,
) -> impl IntoResponse {
	state.requests_total.fetch_add(1, Ordering::Relaxed);
	state.request_body_bytes.fetch_add(body.len() as u64, Ordering::Relaxed);
	let t0 = std::time::Instant::now();
	let req_id = uuid::Uuid::new_v4().to_string();
	let req_id_short = &req_id[..8];

	if state.dev {
		// Log incoming headers
		let mut hdr_log = String::new();
		for (k, v) in headers.iter() {
			let val = v.to_str().unwrap_or("?");
			if k.as_str().to_lowercase() != "authorization" {
				hdr_log.push_str(&format!("  {}: {}", k.as_str(), if val.len() > 80 { &val[..80] } else { val }));
			} else {
				hdr_log.push_str("  authorization: [REDACTED]");
			}
			hdr_log.push('\n');
		}
		tracing::info!(
			id = %req_id_short,
			method = %method,
			path = %path.path(),
			body_len = body.len(),
			headers = %hdr_log,
			">>> REQ"
		);
	}

	// F10: forward the query string too - `path.path()` excludes it, so any
	// OpenAI-compatible endpoint using query params (e.g. `GET
	// /v1/models?limit=5`) had it silently dropped by this catch-all.
	let deepseek_path_and_query = path
		.0
		.path_and_query()
		.map(|pq| pq.as_str())
		.unwrap_or_else(|| path.path())
		.trim_start_matches('/');
	let url = format!("{}/{}", state.api_url.trim_end_matches('/'), deepseek_path_and_query);

	let is_chat_completion = path.path().trim_start_matches('/') == CHAT_COMPLETIONS_PATH.trim_start_matches('/');

	let body_vec = body.to_vec();
	let cache_key = if is_chat_completion {
		cache_key_from_body(&body_vec, state.api_key.expose())
	} else {
		None
	};
	// Check LLM API response cache before upstream call
	if let Some(ck) = cache_key {
		let cached_body = response_cache_get(&state, ck);
		if let Some(cached_body) = cached_body {
			state.cache_hits.fetch_add(1, Ordering::Relaxed);
			// Bytes throughout (report 05 F5): a full upstream round-trip
			// was avoided, so the whole cached response body counts as
			// saved - previously this divided by 4 to estimate a token
			// count while every other `tokens_saved` site counted raw
			// bytes, making the field internally inconsistent by 4x.
			state.tokens_saved.fetch_add(cached_body.len() as u64, Ordering::Relaxed);
			if state.dev {
				tracing::info!(
					id = %req_id_short,
					cached_len = cached_body.len(),
					"<<< CACHE HIT"
				);
			}
			// F18: cache hits used to skip both of these entirely, so
			// `/history` and the latency histogram never saw them - the p50
			// skewed upward (only ever measuring cache MISSES) and the
			// request-history ring buffer under-reported real traffic.
			state.record_latency(t0.elapsed());
			state.record_request(req_id_short, method.as_str(), path.path(), 200, false, t0.elapsed().as_millis());
			return Response::builder()
				.status(StatusCode::OK)
				.header("Content-Type", "application/json; charset=utf-8")
				.header("X-Aphrodite-Cache", "HIT")
				.header("X-Aphrodite-Fill-Pct", {
					let v = state.fill_pct.load(Ordering::Relaxed) as f64 / 100.0;
					if v.is_finite() { format!("{:.1}", v) } else { "0.0".to_string() }
				})
				.body(Body::from(cached_body))
				.unwrap();
		} else {
			state.cache_misses.fetch_add(1, Ordering::Relaxed);
			if state.dev {
				tracing::info!(
					id = %req_id_short,
					"<<< CACHE MISS"
				);
			}
		}
	}
	let mut upstream_result = Err("unreachable".to_string());
	// F17: which counter the final error increments - `upstream_timeouts`
	// used to count every kind of transport failure (connection refused,
	// DNS failure, TLS error) as a "timeout", which is a real metrics lie
	// for anyone diagnosing outages from `/metrics`.
	let mut final_error_was_timeout = false;
	for attempt in 1..=3u32 {
		let req = state
			.client
			.request(method.clone(), &url)
			.header("Content-Type", "application/json; charset=utf-8")
			.header("Accept", "application/json")
			.header("Authorization", format!("Bearer {}", state.api_key.expose()));
		let mut req = req;
		for (key, val) in headers.iter() {
			let k = key.as_str().to_lowercase();
			// Strip (F5/F18): `content-type`/`accept` are forced above already -
			// forwarding the client's own values too would append duplicate
			// headers (reqwest's `header()` appends, it doesn't replace), and
			// strict upstreams reject a duplicated `Content-Type`.
			// `accept-encoding` is stripped entirely (F5): this client is built
			// without gzip/brotli auto-decompression
			// (`Cargo.toml`'s reqwest features), so forwarding a client's
			// `Accept-Encoding: gzip` gets a compressed body back that this
			// proxy can't decode, fails to JSON-parse (compression silently
			// skipped), and returns to the caller as binary garbage labeled
			// `application/json`.
			if k != "host"
				&& k != "authorization"
				&& k != "content-length"
				&& k != "content-type"
				&& k != "accept"
				&& k != "accept-encoding"
				&& !k.starts_with("x-aphrodite-")
			{
				req = req.header(key, val);
			}
		}
		match req.body(body_vec.clone()).send().await {
			Ok(r) => {
				upstream_result = Ok(r);
				break;
			},
			Err(e) => {
				// F17: only retry connect-phase failures - the request never
				// left, so resending is safe. A post-send request TIMEOUT
				// (`e.is_timeout()` after the body was already transmitted)
				// may have been accepted by the upstream; blindly retrying a
				// non-idempotent `POST /v1/chat/completions` risks double
				// token billing, and combined with the 300s per-attempt
				// timeout, 3 blind retries could hold a client for ~15
				// minutes. Non-connect errors now fail fast on the first
				// attempt instead.
				if attempt < 3 && e.is_connect() {
					let base_ms = 100 * 2u64.pow(attempt - 1);
					let jitter = rand::random::<f64>() * 0.5 + 0.75; // 0.75x to 1.25x
					let ms = (base_ms as f64 * jitter) as u64;
					tracing::warn!(attempt, backoff_ms = ms, "upstream retry after connect error: {}", e);
					tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
				} else {
					final_error_was_timeout = e.is_timeout();
					upstream_result = Err(format!("{}", e));
					break;
				}
			},
		}
	}
	match upstream_result {
		Ok(response) => {
			let status = response.status();
			// Track upstream errors by status code
			let status_code = status.as_u16();
			if status_code >= 500 {
				state.upstream_errors_5xx.fetch_add(1, Ordering::Relaxed);
			} else if status_code >= 400 {
				state.upstream_errors_4xx.fetch_add(1, Ordering::Relaxed);
			}
			// F5: capture the full upstream response header map before
			// consuming the body, not just `content-type` - the proxy used
			// to rebuild the response with only `Content-Type`, silently
			// dropping `Retry-After`, `x-ratelimit-*`, `request-id`, etc.
			// that clients rely on for backoff and support.
			let upstream_headers = response.headers().clone();
			let content_type = upstream_headers.get("content-type").cloned();

			// T10 (F6): SSE streaming — text/event-stream responses must be
			// forwarded chunk-by-chunk, not buffered into a single JSON blob
			// (which an SSE client can't parse). Skip compression + cache
			// entirely for this path.
			let is_sse = content_type.as_ref()
				.map(|ct| ct.as_bytes().starts_with(b"text/event-stream"))
				.unwrap_or(false);
			if is_sse {
				let stream = response.bytes_stream();
				if state.dev {
					tracing::info!(id = %req_id_short, status = %status, "<<< STREAM (SSE)");
				}
				state.record_latency(t0.elapsed());
				state.record_request(req_id_short, method.as_str(), path.path(), status.as_u16(), false, t0.elapsed().as_millis());
				let mut builder = Response::builder().status(status);
				builder = copy_upstream_headers(builder, &upstream_headers);
				if let Some(ct) = content_type {
					builder = builder.header("Content-Type", ct);
				}
				builder = builder.header("X-Aphrodite-Streamed", "true");
				return builder.body(Body::from_stream(stream)).unwrap();
			}

			// Buffer the full response body (non-streaming path). Cap at
			// RESPONSE_MAX_BODY_BYTES to prevent a single huge upstream
			// response from exhausting process memory.
			let resp_body = match accumulate_body(response, RESPONSE_MAX_BODY_BYTES).await {
				Ok(b) => b,
				Err(e) => {
					state.record_error(format!("body read: {}", e));
					return (
						StatusCode::BAD_GATEWAY,
						Json(serde_json::json!({"error": format!("body read: {}", e)})),
					)
						.into_response();
				},
			};

			// Track upstream latency (before compression)
			let upstream_elapsed = t0.elapsed().as_micros() as u64;
			state.upstream_latency_micros.fetch_add(upstream_elapsed, Ordering::Relaxed);
			// Track response body bytes
			state.response_body_bytes.fetch_add(resp_body.len() as u64, Ordering::Relaxed);

			// Only compress Chat Completions responses
			let elapsed = t0.elapsed();
			if is_chat_completion && state.ccr.is_some() {
				// Extract headroom budget from inbound headers for compression
				// aggressiveness. `HeaderMap::get` is already case-insensitive
				// (F20), so the second lookup below used to be permanently
				// dead - removed rather than kept as misleading "just in case"
				// code implying case-sensitive matching.
				let headroom_budget = headers.get("x-headroom-budget").and_then(|v| v.to_str().ok());
				if state.dev && headroom_budget.is_some() {
					tracing::info!(
						id = %req_id_short,
						budget = %headroom_budget.unwrap_or(""),
						"headroom budget applied to compression threshold"
					);
				}
				if let Some(compressed) = compress_chat_completion(&state, &resp_body, headroom_budget).await {
					state.requests_compressed.fetch_add(1, Ordering::Relaxed);
					state.record_latency(elapsed);
					state.record_request(
						req_id_short,
						method.as_str(),
						path.path(),
						status.as_u16(),
						true,
						elapsed.as_millis(),
					);
					if state.dev {
						let elapsed = t0.elapsed();
						let comp_len = serde_json::to_vec(&compressed).map(|v| v.len()).unwrap_or(0);
						tracing::info!(
							id = %req_id_short,
							status = %status,
							original_len = resp_body.len(),
							compressed_len = comp_len,
							ratio = format!("{:.1}x", resp_body.len() as f64 / comp_len.max(1) as f64),
							elapsed_ms = elapsed.as_millis(),
							"<<< COMPRESSED"
						);
					}
					let body = serde_json::to_vec(&compressed).unwrap_or_else(|_| resp_body.to_vec());
					// Store in LLM response cache - only successful responses (F2):
					// caching an upstream 4xx/5xx here would replay it as a 200 to
					// every later identical request, and a client would parse the
					// error body as a real chat completion. TTL-stamped (F5,
					// report 06) so it's checked against `response_cache_ttl` on
					// the hit path instead of replaying forever.
					if let Some(ck) = cache_key {
						if status.is_success() && body.len() <= RESPONSE_CACHE_MAX_BODY_BYTES {
							if let Ok(mut cache) = state.response_cache.lock() {
								cache.put(ck, (std::time::Instant::now(), body.clone()));
							}
						}
					}
					let mut builder = Response::builder().status(status);
					builder = copy_upstream_headers(builder, &upstream_headers);
					return builder
						.header("Content-Type", "application/json; charset=utf-8")
						.header("X-Aphrodite-Compressed", "true")
						.header("X-Aphrodite-Cache", "MISS")
						.header("X-Aphrodite-Fill-Pct", {
							let v = state.fill_pct.load(Ordering::Relaxed) as f64 / 100.0;
							if v.is_finite() { format!("{:.1}", v) } else { "0.0".to_string() }
						})
						.body(Body::from(body))
						.unwrap();
				}
			}

			if state.dev {
				let elapsed = t0.elapsed();
				let body_preview = if resp_body.len() > 500 {
					let s = std::str::from_utf8(&resp_body).unwrap_or("?");
					let preview:String = s.char_indices().take_while(|(i, _)| *i < 200).map(|(_, c)| c).collect();
					format!("{}... ({} total)", preview, resp_body.len())
				} else {
					std::str::from_utf8(&resp_body).unwrap_or("?").to_string()
				};
				tracing::info!(
					id = %req_id_short,
					status = %status,
					resp_len = resp_body.len(),
					elapsed_ms = elapsed.as_millis(),
					body = %body_preview,
					"<<< RES"
				);
			}
			// Use already-extracted content_type (fetched before response.bytes())
			state.record_latency(t0.elapsed());
			state.record_request(
				req_id_short,
				method.as_str(),
				path.path(),
				status.as_u16(),
				false,
				t0.elapsed().as_millis(),
			);
			// Store raw response in LLM cache if applicable - success only (F2, see
			// the compressed-path cache write above for the full rationale).
			if let Some(ck) = cache_key {
				if status.is_success() && resp_body.len() <= RESPONSE_CACHE_MAX_BODY_BYTES {
					if let Ok(mut cache) = state.response_cache.lock() {
						cache.put(ck, (std::time::Instant::now(), resp_body.to_vec()));
					}
				}
			}
			let mut builder = Response::builder().status(status);
			builder = copy_upstream_headers(builder, &upstream_headers);
			builder = builder.header("X-Aphrodite-Cache", "MISS");
			builder = builder.header("X-Aphrodite-Fill-Pct", {
				let v = state.fill_pct.load(Ordering::Relaxed) as f64 / 100.0;
				if v.is_finite() { format!("{:.1}", v) } else { "0.0".to_string() }
			});
			if let Some(ct) = content_type {
				builder = builder.header("Content-Type", ct);
			}
			builder.body(Body::from(resp_body)).unwrap()
		},
		Err(e) => {
			if final_error_was_timeout {
				state.upstream_timeouts.fetch_add(1, Ordering::Relaxed);
			} else {
				state.upstream_connect_errors.fetch_add(1, Ordering::Relaxed);
			}
			state.record_latency(t0.elapsed());
			state.record_request(req_id_short, method.as_str(), path.path(), 502, false, t0.elapsed().as_millis());
			state.record_error(format!("upstream: {}", e));
			if state.dev {
				tracing::error!(
					id = %req_id_short,
					error = %e,
					elapsed_ms = t0.elapsed().as_millis(),
					"<<< ERR"
				);
			}
			(
				StatusCode::BAD_GATEWAY,
				Json(serde_json::json!({"error": format!("upstream: {}", e)})),
			)
				.into_response()
		},
	}
}

/// Detect content type for adaptive compression strategy.
fn detect_content_type(content:&str) -> &'static str {
	let first_line = content.lines().next().unwrap_or("");

	// Structured output detection
	if content.starts_with('{') || content.starts_with('[') {
		// Validate JSON before classifying
		if serde_json::from_str::<serde_json::Value>(content).is_err() {
			// Not valid JSON despite starting with { or [ - treat as text
			return "text";
		}
		if content.contains("exit_code") || content.contains("\"status\"") {
			return "tool_output";
		}
		return "json";
	}

	// Code detection - language-specific (before broad error check)
	if content.lines().count() > 3 {
		// Rust - require fn keyword PLUS one of arrow, borrow, or use
		// to distinguish from Python/JavaScript that happens to contain "fn "
		if content.lines().any(|l| {
			let t = l.trim_start();
			t.starts_with("fn ")
				|| t.starts_with("pub fn ")
				|| t.starts_with("async fn ")
				|| t.starts_with("pub async fn ")
				|| t.starts_with("impl ")
				|| t.starts_with("struct ")
				|| t.starts_with("pub struct ")
				|| t.starts_with("enum ")
				|| t.starts_with("pub enum ")
		}) && (content.contains("-> ") || content.contains("&") || content.contains("use "))
		{
			return "code_rust";
		}
		// Python
		if content.contains("def ")
			&& (content.contains("import ")
				|| content.contains("class ")
				|| content.contains("from ")
				|| content.contains("self."))
		{
			return "code_python";
		}
		// Go
		if (content.contains("func ") || content.contains("package ")) && content.contains("import (") {
			return "code_go";
		}
		// JS/TS
		if (content.contains("function ") || content.contains("const ") || content.contains("=> "))
			&& (content.contains("import ") || content.contains("export "))
		{
			return "code_js";
		}
		// Generic code
		if content.contains("fn ")
			|| content.contains("def ")
			|| content.contains("class ")
			|| content.contains("import ")
			|| content.contains("pub fn")
		{
			return "code";
		}
	}

	// Error output - always keep visible
	if first_line.contains("error")
		|| first_line.contains("Error")
		|| first_line.contains("ERROR")
		|| first_line.contains("Traceback")
		|| first_line.contains("panic")
		|| first_line.starts_with("thread '")
	{
		return "error";
	}

	// Build/test output patterns
	if first_line.starts_with("Compiling ")
		|| first_line.starts_with("   Compiling ")
		|| first_line.contains("Finished")
		|| first_line.starts_with("running ")
		|| first_line.starts_with("test ")
	{
		return "build_output";
	}

	// Linter output patterns
	if first_line.starts_with("error[E")
		|| first_line.starts_with("error: ")
		|| first_line.starts_with("warning[")
		|| first_line.starts_with("warning: ")
		|| first_line.contains("|") && (first_line.contains("error") || first_line.contains("warning"))
		|| first_line.contains("mypy")
		|| first_line.contains("clippy")
		|| first_line.contains("eslint")
		|| first_line.contains("tsc ")
	{
		return "linter";
	}

	// Diff output
	if first_line.starts_with("diff --git ")
		|| first_line.starts_with("@@ -")
		|| first_line.starts_with("+++ ")
		|| first_line.starts_with("--- ")
	{
		return "diff";
	}

	// Git output
	if first_line.starts_with("commit ") || first_line.starts_with("On branch ") {
		return "git";
	}

	// Log output - only if content has explicit log markers
	if content.lines().any(|l| {
		let t = l.trim();
		t.starts_with('[')
			&& (t.contains("INFO")
				|| t.contains("WARN")
				|| t.contains("ERROR")
				|| t.contains("DEBUG")
				|| t.contains("TRACE")
				|| t.contains("FATAL")
				|| t.contains("PANIC"))
	}) || content.lines().any(|l| {
		let t = l.trim();
		// Timestamp pattern: ISO-like or syslog-like date at start
		t.starts_with(|c:char| c.is_ascii_digit()) && t.len() > 10 && (t.contains(':') || t.contains('-'))
	}) {
		return "log";
	}
	"text"
}

/// Generate structured metadata for CCR markers based on content type.
/// Returns pipe-safe key=value pairs (max 200 chars, | escaped to /).
fn generate_metadata(content:&str, ct:&str) -> String {
	let line_count = content.lines().count();
	let mut parts:Vec<String> = Vec::new();

	match ct {
		"code_rust" => {
			parts.push("lang=rs".to_string());
			let fns:Vec<&str> = content
				.lines()
				.filter(|l| {
					let t = l.trim_start();
					t.starts_with("fn ") || t.starts_with("pub fn ") || t.starts_with("async fn ")
				})
				.filter_map(|l| {
					let t = l.trim_start();
					let after_fn = t
						.strip_prefix("pub async fn ")
						.or_else(|| t.strip_prefix("pub fn "))
						.or_else(|| t.strip_prefix("async fn "))
						.or_else(|| t.strip_prefix("fn "))?;
					after_fn.split(['(', ' ', '<']).next().filter(|s| !s.is_empty())
				})
				.collect();
			if !fns.is_empty() {
				parts.push(format!("fns={}", fns.join(",")));
			}

			let structs:Vec<&str> = content
				.lines()
				.filter(|l| {
					let t = l.trim_start();
					t.starts_with("struct ") || t.starts_with("pub struct ")
				})
				.filter_map(|l| {
					let t = l.trim_start();
					let after = t
						.strip_prefix("pub struct ")
						.unwrap_or_else(|| t.strip_prefix("struct ").unwrap_or(t));
					after.split(['(', ' ', '<', '{']).next().filter(|s| !s.is_empty())
				})
				.collect();
			if !structs.is_empty() {
				parts.push(format!("structs={}", structs.join(",")));
			}

			// impl blocks: impl TypeName or impl Trait for TypeName
			let impls:Vec<&str> = content
				.lines()
				.filter(|l| {
					let t = l.trim_start();
					t.starts_with("impl ") || t.starts_with("pub impl ")
				})
				.filter_map(|l| {
					let t = l.trim_start();
					let after = t
						.strip_prefix("pub impl ")
						.unwrap_or_else(|| t.strip_prefix("impl ").unwrap_or(t));
					after.split_whitespace().next().map(|w| w.trim_end_matches('<'))
				})
				.collect();
			if !impls.is_empty() {
				parts.push(format!("impls={}", impls.join(",")));
			}

			let traits:Vec<&str> = content
				.lines()
				.filter(|l| {
					let t = l.trim_start();
					t.starts_with("trait ") || t.starts_with("pub trait ")
				})
				.filter_map(|l| {
					let t = l.trim_start();
					let after = t
						.strip_prefix("pub trait ")
						.unwrap_or_else(|| t.strip_prefix("trait ").unwrap_or(t));
					after.split([' ', '<', '{']).next().filter(|s| !s.is_empty())
				})
				.collect();
			if !traits.is_empty() {
				parts.push(format!("traits={}", traits.join(",")));
			}

			parts.push(format!("ln={}", line_count));
		},
		"code_python" => {
			parts.push("lang=py".to_string());
			let fns:Vec<&str> = content
				.lines()
				.filter(|l| {
					let t = l.trim_start();
					t.starts_with("def ") || t.starts_with("async def ")
				})
				.filter_map(|l| {
					let t = l.trim_start();
					let after = t
						.strip_prefix("async def ")
						.unwrap_or_else(|| t.strip_prefix("def ").unwrap_or(t));
					after.split(['(', ' ', ':']).next().filter(|s| !s.is_empty())
				})
				.collect();
			if !fns.is_empty() {
				parts.push(format!("fns={}", fns.join(",")));
			}
			let classes:Vec<&str> = content
				.lines()
				.filter(|l| {
					let t = l.trim_start();
					t.starts_with("class ")
				})
				.filter_map(|l| {
					let t = l.trim_start();
					let after = t.strip_prefix("class ")?;
					after.split(['(', ' ', ':']).next().filter(|s| !s.is_empty())
				})
				.collect();
			if !classes.is_empty() {
				parts.push(format!("classes={}", classes.join(",")));
			}
			let imports:Vec<&str> = content
				.lines()
				.filter(|l| {
					let t = l.trim_start();
					t.starts_with("import ") || t.starts_with("from ")
				})
				.filter_map(|l| {
					let t = l.trim_start();
					if let Some(rest) = t.strip_prefix("import ") {
						rest.split([' ', ',', ';']).next().filter(|s| !s.is_empty())
					} else {
						t.strip_prefix("from ")?.split(' ').next().filter(|s| !s.is_empty())
					}
				})
				.collect();
			if !imports.is_empty() {
				parts.push(format!("imports={}", imports.join(",")));
			}
			// Decorators: @route, @dataclass, @staticmethod, etc.
			let decorators:Vec<&str> = content
				.lines()
				.filter(|l| {
					let t = l.trim_start();
					t.starts_with('@')
				})
				.filter_map(|l| {
					let t = l.trim_start();
					let name = &t[1..];
					name.split(['(', ' ']).next().filter(|s| !s.is_empty())
				})
				.collect();
			if !decorators.is_empty() {
				parts.push(format!("decorators={}", decorators.join(",")));
			}
			parts.push(format!("ln={}", line_count));
		},
		"code_go" => {
			parts.push("lang=go".to_string());
			let fns:Vec<&str> = content
				.lines()
				.filter(|l| {
					let t = l.trim_start();
					t.starts_with("func ")
				})
				.filter_map(|l| {
					let t = l.trim_start();
					let after = t.strip_prefix("func ")?;
					after.split(['(', ' ']).next().filter(|s| !s.is_empty())
				})
				.collect();
			if !fns.is_empty() {
				parts.push(format!("fns={}", fns.join(",")));
			}
			parts.push(format!("ln={}", line_count));
		},
		"code_js" => {
			parts.push("lang=js".to_string());
			let fns:Vec<&str> = content
				.lines()
				.filter(|l| {
					let t = l.trim_start();
					t.starts_with("function ") || t.starts_with("const ")
				})
				.filter_map(|l| {
					let t = l.trim_start();
					if let Some(rest) = t.strip_prefix("function ") {
						rest.split(['(', ' ']).next().filter(|s| !s.is_empty())
					} else {
						t.strip_prefix("const ")?
							.split([' ', '=', ':'])
							.next()
							.filter(|s| !s.is_empty())
					}
				})
				.collect();
			if !fns.is_empty() {
				parts.push(format!("fns={}", fns.join(",")));
			}
			parts.push(format!("ln={}", line_count));
		},
		"code" => {
			parts.push("lang=gen".to_string());
			// Try to extract function-like signatures from unknown code
			let sigs:Vec<&str> = content
				.lines()
				.filter(|l| {
					let t = l.trim_start();
					t.starts_with("fn ")
						|| t.starts_with("def ")
						|| t.starts_with("func ")
						|| t.starts_with("function ")
						|| t.starts_with("class ")
						|| t.starts_with("struct ")
				})
				.filter_map(|l| {
					let t = l.trim_start();
					let after = t
						.strip_prefix("fn ")
						.or_else(|| t.strip_prefix("def "))
						.or_else(|| t.strip_prefix("func "))
						.or_else(|| t.strip_prefix("function "))
						.or_else(|| t.strip_prefix("class "))
						.or_else(|| t.strip_prefix("struct "))?;
					after.split(['(', ' ']).next()
				})
				.collect();
			if !sigs.is_empty() {
				parts.push(format!("sigs={}", sigs.join(",")));
			}
			parts.push(format!("ln={}", line_count));
		},
		"error" => {
			let mut trace = String::new();
			for l in content.lines() {
				let t = l.trim();
				let ext_pos = t.find(".rs:").or_else(|| t.find(".py:")).or_else(|| t.find(".go:"));
				if let Some(pos) = ext_pos {
					// `pos` itself is a valid boundary (`.find` on ASCII
					// patterns), but `start`/`end` are arbitrary byte offsets
					// from it and can land mid-codepoint on non-ASCII lines
					// (e.g. a CJK comment before ".rs:") - snap both to the
					// nearest valid boundary rather than panicking on `t[..]`.
					let mut start = pos.saturating_sub(12);
					while start > 0 && !t.is_char_boundary(start) {
						start -= 1;
					}
					let mut end = (pos + 40).min(t.len());
					while end < t.len() && !t.is_char_boundary(end) {
						end += 1;
					}
					trace = t[start..end].to_string();
					break;
				}
			}
			if !trace.is_empty() {
				parts.push(format!("trace={}", trace.replace('|', "/")));
			}
			let msg = content.lines().find(|l| l.contains("Error:") || l.contains("error[")).map(|l| {
				let t = l.trim();
				let idx = t.find("Error:").or_else(|| t.find("error[")).unwrap_or(0);
				t[idx..].chars().take(80).collect::<String>().replace('|', "/")
			});
			if let Some(m) = msg {
				parts.push(format!("msg={}", m));
			} else {
				let fl = content.lines().next().unwrap_or("").trim();
				if !fl.is_empty() {
					parts.push(format!("msg={}", fl.chars().take(80).collect::<String>().replace('|', "/")));
				}
			}
			let err_count = content
				.lines()
				.filter(|l| l.contains("error") || l.starts_with("thread '"))
				.count();
			if err_count > 0 {
				parts.push(format!("N_errors={}", err_count));
			}
		},
		"diff" => {
			let files = content.lines().filter(|l| l.starts_with("diff --git ")).count();
			if files > 0 {
				parts.push(format!("files={}", files));
			}
			let adds = content.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
			let dels = content.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
			if adds > 0 {
				parts.push(format!("adds={}", adds));
			}
			if dels > 0 {
				parts.push(format!("dels={}", dels));
			}
		},
		"git" => {
			for l in content.lines() {
				let t = l.trim();
				if let Some(rest) = t.strip_prefix("On branch ") {
					parts.push(format!("branch={}", rest.trim().replace('|', "/")));
					break;
				}
			}
			if !parts.iter().any(|p| p.starts_with("branch=")) {
				for l in content.lines() {
					let t = l.trim();
					if !t.is_empty() && !t.starts_with("* ") && !t.starts_with("  ") {
						parts.push(format!("branch={}", t.chars().take(40).collect::<String>().replace('|', "/")));
						break;
					}
				}
			}
			let commits = content
				.lines()
				.filter(|l| l.starts_with("commit ") || l.trim().starts_with("* ") || l.contains("commit"))
				.count();
			if commits > 0 {
				parts.push(format!("commits={}", commits));
			}
		},
		"build_output" => {
			if content.contains("error") || content.contains("aborting") {
				parts.push("status=FAIL".to_string());
			} else {
				parts.push("status=OK".to_string());
			}
			let files = content
				.lines()
				.filter(|l| l.starts_with("Compiling ") || l.contains(" Compiling "))
				.count();
			if files > 0 {
				parts.push(format!("files={}", files));
			}
			let first_err = content
				.lines()
				.find(|l| l.contains("error[") || l.contains("Error:"))
				.map(|l| l.trim().chars().take(80).collect::<String>().replace('|', "/"));
			if let Some(e) = first_err {
				parts.push(format!("first_err={}", e));
			}
		},
		"log" => {
			for l in content.lines() {
				let t = l.trim();
				for level in &["ERROR", "WARN", "WARNING", "INFO", "DEBUG", "TRACE", "FATAL", "PANIC"] {
					if t.contains(level) {
						parts.push(format!("level={}", level.to_lowercase()));
						break;
					}
				}
				if parts.iter().any(|p| p.starts_with("level=")) {
					break;
				}
			}
			let last_line = content.lines().last().unwrap_or("").trim().chars().take(60).collect::<String>();
			if !last_line.is_empty() {
				parts.push(format!("last={}", last_line.replace('|', "/")));
			}
			parts.push(format!("ln={}", line_count));
		},
		"linter" => {
			let files_linted = content
				.lines()
				.filter(|l| {
					(l.contains(".rs:") || l.contains(".py:") || l.contains(".go:") || l.contains(".ts:"))
						&& (l.contains("error") || l.contains("warning"))
				})
				.count();
			if files_linted > 0 {
				parts.push(format!("files={}", files_linted));
			}
			let first_err = content
				.lines()
				.find(|l| l.contains("error[") || l.contains("Error:") || l.starts_with("error: "))
				.map(|l| l.trim().chars().take(80).collect::<String>().replace('|', "/"));
			if let Some(e) = first_err {
				parts.push(format!("first_err={}", e));
			}
			parts.push(format!("ln={}", line_count));
		},
		"json" | "tool_output" => {
			// Extract unique top-level JSON keys via simple scan
			// Look for '"key_name":' patterns without regex
			let mut keys:Vec<String> = Vec::new();
			for l in content.lines() {
				// Find a sequence: '"' + some chars + '":'
				let bytes = l.as_bytes();
				let mut i = 0;
				while i + 3 < bytes.len() {
					if bytes[i] == b'"' {
						let start = i + 1;
						let mut end = start;
						while end < bytes.len() && bytes[end] != b'"' {
							end += 1;
						}
						if end < bytes.len() && end + 2 < bytes.len() && bytes[end + 1] == b':' {
							let key = &l[start..end];
							if !key.starts_with('_') && !keys.contains(&key.to_string()) {
								keys.push(key.to_string());
								if keys.len() >= 10 {
									break;
								}
							}
						}
						i = end + 1;
					} else {
						i += 1;
					}
				}
				if keys.len() >= 10 {
					break;
				}
			}
			if !keys.is_empty() {
				parts.push(format!("keys={}", keys.join(",")));
			}
			// Estimate entries count from array-like patterns
			let entries = content
				.lines()
				.filter(|l| {
					let t = l.trim();
					t.starts_with('{') || t.starts_with('"') || t.starts_with('[')
				})
				.count();
			if entries > 1 {
				parts.push(format!("entries={}", entries));
			}
		},
		"text" => {
			parts.push(format!("ln={}", line_count));
		},
		_ => {
			parts.push(format!("ln={}", line_count));
		},
	}

	// Build final string: ;-separated key=value pairs (; safe within CCR marker's |
	// delimiters). Comma separates list items within values. Max 400 chars
	// (coding-tuned: enough for ~25 functions).
	let result = parts.join(";").replace('\n', " ").replace('\r', "");
	let truncated:String = result.chars().take(400).collect();
	truncated.trim_end_matches([';', ' ', ',']).to_string()
}

// ── CCR output template (editable) ────────────────────────────────
/// Edit this function to change the layout the LLM sees when content
/// is compressed. Three-line format by default: preview, structure, marker.
fn format_ccr_output(preview:&str, ct:&str, metadata:&str, center:Option<&str>, hash:&str, size:usize) -> String {
	let center_seg = center.map(|c| format!(";center={c}")).unwrap_or_default();
	format!("{preview}\n[{ct}: {metadata}{center_seg}]\n<<<CCR:{hash}|{ct}|{size}>>>")
}

/// Build a smart content-type-aware preview for the CCR output.
///
/// Returns the most informative excerpt based on content type:
/// - Code: first 3 lines (imports + first signature)
/// - Error: the actual error line, not the traceback header
/// - Diff: first file changed
/// - JSON: key count summary
/// - Default: first line, ~250 chars
fn build_preview(content:&str, ct:&str) -> String {
	match ct {
		"code_rust" | "code_python" | "code_go" | "code_js" | "code_ts" | "code_sh" | "code" => {
			// Code: structure-map preview - extract fn/def/class/struct sigs
			let mut fns:Vec<&str> = Vec::new();
			let mut structs:Vec<&str> = Vec::new();
			let mut impls:Vec<&str> = Vec::new();
			let mut classes:Vec<&str> = Vec::new();
			let mut budget:usize = 280;

			for line in content.lines() {
				if budget == 0 {
					break;
				}
				let trimmed = line.trim();
				if trimmed.is_empty() {
					continue;
				}

				// Rust patterns
				if ct == "code_rust" || ct == "code" {
					if trimmed.strip_prefix("fn ").is_some() {
						let sig:String = trimmed.chars().take(58).collect();
						fns.push(trimmed); // store ref, build later
						budget = budget.saturating_sub(sig.len() + 2);
					} else if trimmed.strip_prefix("pub fn ").is_some() {
						let sig:String = trimmed.chars().take(58).collect();
						fns.push(trimmed);
						budget = budget.saturating_sub(sig.len() + 2);
					} else if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
						let s:String = trimmed.chars().take(50).collect();
						structs.push(trimmed);
						budget = budget.saturating_sub(s.len() + 2);
					} else if trimmed.starts_with("impl ") {
						let s:String = trimmed.chars().take(50).collect();
						impls.push(trimmed);
						budget = budget.saturating_sub(s.len() + 2);
					}
				}
				// Python patterns
				if ct == "code_python" || ct == "code" {
					if (trimmed.starts_with("def ") || trimmed.starts_with("async def ")) && trimmed.ends_with(':') {
						let s:String = trimmed.chars().take(58).collect();
						fns.push(trimmed);
						budget = budget.saturating_sub(s.len() + 2);
					} else if trimmed.starts_with("class ") && trimmed.ends_with(':') {
						let s:String = trimmed.chars().take(50).collect();
						classes.push(trimmed);
						budget = budget.saturating_sub(s.len() + 2);
					}
				}
				// Go patterns
				if ct == "code_go" && trimmed.starts_with("func ") {
					let s:String = trimmed.chars().take(58).collect();
					fns.push(trimmed);
					budget = budget.saturating_sub(s.len() + 2);
				}
			}

			// Build summary line: [code_rust:3fns|2structs|1impl crate::proxy]
			let mut parts:Vec<String> = Vec::new();
			if !fns.is_empty() {
				parts.push(format!("{}fns", fns.len()));
			}
			if !structs.is_empty() {
				parts.push(format!("{}structs", structs.len()));
			}
			if !impls.is_empty() {
				parts.push(format!("{}impls", impls.len()));
			}
			if !classes.is_empty() {
				parts.push(format!("{}classes", classes.len()));
			}
			let summary = if parts.is_empty() { "?".to_string() } else { parts.join("|") };

			// Show first 2 signatures inline
			let sig_previews:Vec<String> = fns.iter().take(2).map(|s| s.chars().take(56).collect::<String>()).collect();
			let sig_str = sig_previews.join("; ");

			let lines = content.lines().count();
			format!("[{ct}:{summary} {sig_str} {lines}L]").chars().take(300).collect()
		},
		"error" => {
			// Error: find the actual error line, skip traceback noise
			let err_line = content
				.lines()
				.find(|l| l.contains("Error:") || l.contains("error[") || l.contains("panicked"))
				.unwrap_or_else(|| content.lines().next().unwrap_or(""));
			err_line.chars().take(300).collect()
		},
		"diff" => {
			// Diff: show which files changed
			let files:Vec<&str> = content.lines().filter(|l| l.starts_with("diff --git ")).take(2).collect();
			if files.is_empty() {
				content.lines().next().unwrap_or("").chars().take(200).collect()
			} else {
				files.join("\n").chars().take(300).collect()
			}
		},
		"json" | "tool_output" => {
			// JSON: first line + key count
			let first = content.lines().next().unwrap_or("");
			let key_count = content.matches("\":").count();
			format!("{} … {} keys", first.chars().take(150).collect::<String>(), key_count)
		},
		"build_output" => {
			// Build: show status line
			content
				.lines()
				.find(|l| l.contains("Compiling") || l.contains("Finished") || l.contains("error"))
				.unwrap_or_else(|| content.lines().next().unwrap_or(""))
				.chars()
				.take(250)
				.collect()
		},
		_ => {
			// Default: first line, ~250 chars
			content.lines().next().unwrap_or("").chars().take(250).collect()
		},
	}
}

/// Create a CCR marker with preview and structure for the LLM.
///
/// Uses [`format_ccr_output`] for the output layout. The LLM reads the
/// preview + structure first, then decides whether to call
/// aphrodite_retrieve for the full content.
fn smart_marker(hash:&str, content:&str, ct:&str, center:Option<&str>) -> String {
	let size = content.len();
	let metadata = generate_metadata(content, ct);
	let preview = build_preview(content, ct);
	format_ccr_output(&preview, ct, &metadata, center, hash, size)
}

/// Cache-mode CCR output - preview + marker, same template.
fn cache_marker(hash:&str, content:&str, ct:&str, center:Option<&str>) -> String {
	let size = content.len();
	let preview:String = content.chars().take(512).collect();
	format_ccr_output(&preview, ct, "", center, hash, size)
}

/// Compress a Chat Completions API response with smart markers.
async fn compress_chat_completion(
	state:&AppState,
	resp_body:&[u8],
	headroom_budget:Option<&str>,
) -> Option<serde_json::Value> {
	let mut response:serde_json::Value = serde_json::from_slice(resp_body).ok()?;
	let choices = response.get_mut("choices")?.as_array_mut()?;
	let base_threshold = state.compress_threshold(); // floor threshold for all types

	// Headroom budget: lower values compress more aggressively.
	// Coding-tuned: smooth linear curve from 0.50 (empty) to 1.0 (full).
	// Never below 0.5× - semantics and tool chains are worth the tokens.
	let budget_mult = headroom_budget
		.and_then(|b| {
			let val:f64 = b.parse().ok()?;
			// Linear interpolation: 0.50 + (fill% * 0.50), clamped [0.50, 1.0]
			Some((0.50 + (val / 100.0) * 0.50).clamp(0.50, 1.0))
		})
		.unwrap_or(1.0);
	let mut did_compress = false;

	for choice in choices {
		let message = choice.get_mut("message")?;

		// Compress text content with smart markers
		if let Some(content_val) = message.get_mut("content") {
			if let Some(content) = content_val.as_str() {
				let ct = detect_content_type(content);
				let threshold = (state.threshold_for(ct).max(base_threshold) as f64 * budget_mult) as usize;
				if content.len() > threshold {
					if let Some(ccr) = &state.ccr {
						let hash = compute_key(content.as_bytes());
						// F4: only replace `content` with a marker if the content is
						// actually retrievable under `hash` - either it was already
						// there (cache hit) or this `put` succeeded. A failed put
						// (store full/locked/panicked) must NOT be followed by
						// swapping the response for an unresolvable marker - that
						// would permanently destroy content that never reached the
						// client any other way.
						let stored = if ccr_get(ccr, &hash).await.is_some() {
							state.ccr_hits.fetch_add(1, Ordering::Relaxed);
							true
						} else {
							state.ccr_misses.fetch_add(1, Ordering::Relaxed);
							let ok = ccr_put(ccr, &hash, content).await;
							if ok {
								state.ccr_created.fetch_add(1, Ordering::Relaxed);
							} else {
								tracing::error!(hash = %hash, "ccr_put failed - leaving content uncompressed to avoid data loss");
							}
							ok
						};
						if stored {
							let (compressed, orig_len) = {
								let compressed = match state.mode {
									ProxyMode::Cache => cache_marker(&hash, content, ct, None),
									ProxyMode::Token => smart_marker(&hash, content, ct, None),
								};
								let len = content.len();
								state.record_compression(ct);
								(compressed, len)
							};
							let marker_len = compressed.len();
							// Savings = bytes actually removed from the response
							// (original content minus the rendered marker that
							// replaces it), not the bare hash length - the marker
							// is hundreds of chars longer than the 40-char hash,
							// so subtracting only `hash.len()` overstated savings
							// (report 05 F5). Unit is bytes throughout - see
							// `tokens_saved`'s field doc for the naming caveat.
							state
								.tokens_saved
								.fetch_add(orig_len.saturating_sub(marker_len) as u64, Ordering::Relaxed);
							*content_val = serde_json::Value::String(compressed);
							did_compress = true;
							state.update_compression_ratio(orig_len, marker_len);
						}
					}
				} else if content.len() > state.inline_ccr_threshold() {
					// Below compression threshold but above inline threshold: store in inline_ccr
					// so later retrievals can find tiny entries without a backend round-trip.
					let hash = compute_key(content.as_bytes());
					if let Ok(mut map) = state.inline_ccr.lock() {
						if map.contains(&hash) {
							state.inline_ccr_hits.fetch_add(1, Ordering::Relaxed);
						} else {
							state.inline_ccr_misses.fetch_add(1, Ordering::Relaxed);
							map.put(hash, content.to_string());
						}
					}
				}
			}
		}

		// Compress tool call outputs
		if let Some(tool_calls_val) = message.get_mut("tool_calls") {
			if let Some(arr) = tool_calls_val.as_array_mut() {
				for tc in arr.iter_mut() {
					if let Some(func) = tc.get_mut("function") {
						if let Some(args) = func.get_mut("arguments") {
							if let Some(args_str) = args.as_str() {
								let args_owned = args_str.to_string(); // drop borrow before mutation
								let ct = detect_content_type(&args_owned);
								let threshold =
									(state.threshold_for(ct).max(base_threshold) as f64 * budget_mult) as usize;
								if args_owned.len() > threshold {
									if let Some(ccr) = &state.ccr {
										let hash = compute_key(args_owned.as_bytes());
										// F4: same rule as the message.content branch
										// above - only replace with a marker if the
										// content is actually retrievable.
										let stored = if ccr_get(ccr, &hash).await.is_some() {
											state.ccr_hits.fetch_add(1, Ordering::Relaxed);
											true
										} else {
											state.ccr_misses.fetch_add(1, Ordering::Relaxed);
											let ok = ccr_put(ccr, &hash, &args_owned).await;
											if ok {
												state.ccr_created.fetch_add(1, Ordering::Relaxed);
											} else {
												tracing::error!(hash = %hash, "ccr_put failed - leaving tool_call arguments uncompressed to avoid data loss");
											}
											ok
										};
										if stored {
											let (compressed, orig_len) = {
												let compressed = smart_marker(&hash, &args_owned, ct, None);
												let len = args_owned.len();
												state.record_compression(ct);
												(compressed, len)
											};
											let marker_len = compressed.len();
											// Savings = bytes removed by the marker
											// replacement, not the bare hash length
											// (report 05 F5) - see the sibling fix
											// above for `message.content`.
											state.tokens_saved.fetch_add(
												orig_len.saturating_sub(marker_len) as u64,
												Ordering::Relaxed,
											);
											*args = serde_json::Value::String(compressed);
											did_compress = true;
											state.update_compression_ratio(orig_len, marker_len);
										}
									}
								} else if args_owned.len() > state.inline_ccr_threshold() {
									let hash = compute_key(args_owned.as_bytes());
									if let Ok(mut map) = state.inline_ccr.lock() {
										if map.contains(&hash) {
											state.inline_ccr_hits.fetch_add(1, Ordering::Relaxed);
										} else {
											state.inline_ccr_misses.fetch_add(1, Ordering::Relaxed);
											map.put(hash, args_owned);
										}
									}
								}
							}
						}
					}
				}

				// Tool injection removed - aphrodite_retrieve is registered by
				// the Python plugin. Injecting into the response tool_calls
				// array was incorrect (Bug 18).
			}
		}
	}

	if did_compress { Some(response) } else { None }
}

// ── Tool relay handler ───────────────────────────────────────────────

/// `POST /tool_relay` - dispatches a Hermes-side tool call (see
/// [`execute_tool_relay`]). Runs synchronously unless the request carries
/// an `https://` `callback_url`, in which case it's spawned onto
/// `task_tracker` and the result is POSTed back later instead of returned
/// inline.
pub async fn handle_tool_relay(
	State(state):State<Arc<AppState>>,
	Json(req):Json<ToolRelayRequest>,
) -> impl IntoResponse {
	state.tool_relay_calls.fetch_add(1, Ordering::Relaxed);
	tracing::info!(tool = %req.tool, "tool_relay");

	// Validate aphrodite_retrieve: requests with only `query` and no `hash` are
	// invalid and must return 400 BAD_REQUEST instead of silently passing through.
	if req.tool == "aphrodite_retrieve" && req.params.get("hash").and_then(|v| v.as_str()).is_none() {
		return (
			StatusCode::BAD_REQUEST,
			Json(ToolRelayResponse {
				success:false,
				result:None,
				error:Some(
					"`hash` is required for 💋/aphrodite_retrieve. Requests with only `query` and no `hash` are \
					 invalid."
						.into(),
				),
				async_call:false,
			}),
		)
			.into_response();
	}

	if let Some(cb) = &req.callback_url {
		// SSRF protection: only https:// URLs allowed
		let parsed_url = match url::Url::parse(cb) {
			Ok(u) if u.scheme() == "https" => u,
			_ => {
				// F13: report the rejection honestly instead of `success:true`
				// with nothing executed - a caller passing e.g. a loopback
				// `http://` callback previously got told it worked and then
				// waited forever for a callback that would never arrive.
				tracing::warn!(callback_url = %cb, "tool_relay callback rejected: only https scheme allowed");
				return (
					StatusCode::BAD_REQUEST,
					Json(ToolRelayResponse {
						success:false,
						result:None,
						error:Some("callback_url must use the https scheme".into()),
						async_call:false,
					}),
				)
					.into_response();
			},
		};
		let tracker = state.task_tracker.clone();
		let state = state.clone();
		let tool = req.tool.clone();
		let params = req.params.clone();
		let cb = parsed_url.to_string();
		tracker.spawn(async move {
			let result = execute_tool_relay(&state, &tool, &params).await;
			// F13: the execute result itself (success/failure of the tool
			// call) was never counted on this async path - only the
			// synchronous path below incremented these, so
			// `tool_relay_success + tool_relay_failure` silently diverged
			// from `tool_relay_calls` for every async callback request.
			if result.is_ok() {
				state.tool_relay_success.fetch_add(1, Ordering::Relaxed);
			} else {
				state.tool_relay_failure.fetch_add(1, Ordering::Relaxed);
			}
			let _ = state
				.client
				.post(&cb)
				.json(&result)
				.timeout(Duration::from_secs(5))
				.send()
				.await;
		});
		return Json(ToolRelayResponse { success:true, result:None, error:None, async_call:true }).into_response();
	}

	match execute_tool_relay(&state, &req.tool, &req.params).await {
		Ok(val) => {
			state.tool_relay_success.fetch_add(1, Ordering::Relaxed);
			Json(ToolRelayResponse { success:true, result:Some(val), error:None, async_call:false }).into_response()
		},
		Err(e) => {
			state.tool_relay_failure.fetch_add(1, Ordering::Relaxed);
			Json(ToolRelayResponse { success:false, result:None, error:Some(e), async_call:false }).into_response()
		},
	}
}

/// Dispatch a single tool-relay call by name (`aphrodite_retrieve`,
/// `aphrodite_compress`, `aphrodite_list`) - the actual work behind
/// `POST /tool_relay`, called from [`handle_tool_relay`].
async fn execute_tool_relay(
	state:&AppState,
	tool:&str,
	params:&serde_json::Value,
) -> Result<serde_json::Value, String> {
	match tool {
		"aphrodite_retrieve" => {
			let hash_raw = params.get("hash").and_then(|v| v.as_str()).ok_or("missing hash")?;
			// Strip a `|type|size` marker-body suffix and surrounding
			// whitespace (report 05 F3) - an LLM sometimes echoes the full
			// marker body back as the hash argument instead of the bare
			// hash, and the lookups below (inline_ccr, CCR backend) are
			// exact-match only.
			let hash = crate::marker::normalize_hash(hash_raw);
			// Check inline_ccr first (no round-trip needed for tiny entries)
			if let Ok(mut map) = state.inline_ccr.lock() {
				if let Some(content) = map.get(hash) {
					state.inline_ccr_hits.fetch_add(1, Ordering::Relaxed);
					return Ok(serde_json::json!({"found": true, "content": content.clone()}));
				}
			}
			state.inline_ccr_misses.fetch_add(1, Ordering::Relaxed);
			// Fallback to CCR store
			if let Some(ccr) = &state.ccr {
				match ccr_get(ccr, hash).await {
					Some(content) => Ok(serde_json::json!({"found": true, "content": content})),
					None => Ok(serde_json::json!({"found": false})),
				}
			} else {
				Err("CCR not enabled".into())
			}
		},
		"aphrodite_compress" => {
			let content = params.get("content").and_then(|v| v.as_str()).ok_or("missing content")?;
			let center = params.get("_ccr_center").and_then(|v| v.as_str());
			let hash = compute_key(content.as_bytes());
			let size = content.len();
			if size < state.inline_ccr_threshold() {
				// Tiny content: store inline for the fast path (no CCR backend
				// round-trip), AND to the durable backend when one is
				// configured (report 06 F5) - the inline map is a 1024-entry
				// process-memory LRU, so a busy session can evict this entry
				// within minutes even though the marker handed back looks
				// exactly like a durable one; without the durable copy,
				// `aphrodite_retrieve` after eviction (or a proxy restart)
				// returns `{"found": false}` for a marker the model was told
				// is resolvable. Best-effort: a failed durable put doesn't
				// fail this call since the inline copy still serves reads
				// until it's evicted.
				if let Ok(mut map) = state.inline_ccr.lock() {
					if map.contains(&hash) {
						state.inline_ccr_hits.fetch_add(1, Ordering::Relaxed);
					} else {
						state.inline_ccr_misses.fetch_add(1, Ordering::Relaxed);
						map.put(hash.clone(), content.to_string());
					}
				}
				if let Some(ccr) = &state.ccr {
					ccr_put(ccr, &hash, content).await;
				}
				Ok(serde_json::json!({
					"compressed": smart_marker(&hash, content, "compress", center),
					"hash": hash,
					"original_size": size
				}))
			} else if let Some(ccr) = &state.ccr {
				// F4: don't hand back a marker for content that failed to store.
				if !ccr_put(ccr, &hash, content).await {
					return Err("failed to store content in CCR backend".into());
				}
				let compressed = smart_marker(&hash, content, "compress", center);
				// Savings = bytes removed by the marker replacement, not the
				// bare hash length (report 05 F5).
				state
					.tokens_saved
					.fetch_add(size.saturating_sub(compressed.len()) as u64, Ordering::Relaxed);
				Ok(serde_json::json!({
					"compressed": compressed,
					"hash": hash,
					"original_size": size
				}))
			} else {
				Err("CCR not enabled".into())
			}
		},
		"aphrodite_list" => {
			let entries = match &state.ccr {
				Some(ccr) => ccr_len(ccr).await,
				None => 0,
			};
			Ok(serde_json::json!({
				"entries": entries,
				"backend": match state.mode {
					ProxyMode::Cache => "in_memory",
					ProxyMode::Token => "sqlite",
				},
			}))
		},
		_ => Err(format!("Unknown tool: {}", tool)),
	}
}

// ── Programmatic CCR handlers ────────────────────────────────────────

/// `POST /ccr/create` - stores content directly into the CCR backend,
/// bypassing the Chat Completions compression path. Accepts either a JSON
/// [`CcrCreateRequest`] body or a raw octet-stream (treated as the content
/// itself, hashed for the key). Fires the `notify_url` webhook on success
/// if configured.
pub async fn handle_ccr_create(
	State(state):State<Arc<AppState>>,
	headers:axum::http::HeaderMap,
	body:Bytes,
) -> impl IntoResponse {
	let content_type = headers.get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");

	// Support both JSON and raw octet-stream bodies
	if content_type.contains("json") {
		// Parse as JSON CcrCreateRequest
		match serde_json::from_slice::<CcrCreateRequest>(&body) {
			Ok(req) => {
				let original_size = req.content.len();
				let hash = req.key.unwrap_or_else(|| compute_key(req.content.as_bytes()));

				// F14: report unavailability instead of a fabricated success -
				// previously this endpoint returned a hash + savings ratio even
				// when `state.ccr` was `None` (e.g. token mode with
				// `--no-ccr-marker`), so nothing was ever stored and every
				// later `/retrieve` of that hash 404s. Mirrors
				// `handle_ccr_delete`'s existing `None` branch.
				let ccr = match &state.ccr {
					Some(ccr) => ccr,
					None => {
						return (
							StatusCode::SERVICE_UNAVAILABLE,
							Json(serde_json::json!({"error": "CCR not enabled"})),
						)
							.into_response();
					},
				};
				if !ccr_put(ccr, &hash, &req.content).await {
					return (
						StatusCode::INTERNAL_SERVER_ERROR,
						Json(serde_json::json!({"error": "failed to store content in CCR backend"})),
					)
						.into_response();
				}
				state.ccr_created.fetch_add(1, Ordering::Relaxed);
				// Unlike the chat-completion/tool-relay paths, this
				// endpoint's documented wire contract IS the bare hash
				// (see `compressed_size`/`marker_size` below and
				// `test_ccr_create_response_serde_shape`) - no marker is
				// rendered here, so `hash.len()` is the correct
				// subtractee, not an approximation (report 05 F5).
				state
					.tokens_saved
					.fetch_add(original_size.saturating_sub(hash.len()) as u64, Ordering::Relaxed);
				state.requests_compressed.fetch_add(1, Ordering::Relaxed);

				if let Some(notify_url) = &state.notify_url {
					let notification = CcrNotification {
						event:"ccr_created".into(),
						hash:hash.clone(),
						created_at:std::time::SystemTime::now()
							.duration_since(std::time::UNIX_EPOCH)
							.unwrap_or_default()
							.as_secs(),
						ttl:req.ttl_seconds.unwrap_or(3600),
						tags:req.tags.unwrap_or_default(),
					};
					let tracker = state.task_tracker.clone();
					let client = state.client.clone();
					let url = notify_url.clone();
					let key = state.notify_key.clone();
					let state_clone = state.clone();
					tracker.spawn(async move {
						let mut req = client.post(&url).json(&notification);
						if let Some(k) = &key {
							req = req.header("Authorization", format!("Bearer {k}"));
						}
						match req.timeout(Duration::from_secs(5)).send().await {
							Ok(r) if r.status().is_success() => {
								state_clone.notify_success.fetch_add(1, Ordering::Relaxed);
							},
							_ => {
								state_clone.notify_failure.fetch_add(1, Ordering::Relaxed);
							},
						}
					});
				}

				let compressed_size = hash.len();
				Json(CcrCreateResponse {
					hash,
					token_savings_ratio:if original_size > 0 {
						original_size as f64 / compressed_size.max(1) as f64
					} else {
						1.0
					},
					original_size,
					compressed_size,
					marker_size:compressed_size,
				})
				.into_response()
			},
			Err(e) => {
				(
					StatusCode::BAD_REQUEST,
					Json(serde_json::json!({"error": format!("invalid JSON: {}", e)})),
				)
					.into_response()
			},
		}
	} else {
		// Treat raw body as content directly
		let content = match String::from_utf8(body.to_vec()) {
			Ok(c) => c,
			Err(_) => {
				return (
					StatusCode::BAD_REQUEST,
					Json(serde_json::json!({"error": "invalid UTF-8 in body"})),
				)
					.into_response();
			},
		};
		let original_size = content.len();
		let hash = compute_key(content.as_bytes());

		// F14/F4: same rules as the JSON-body branch above - 503 when CCR
		// isn't enabled, 500 (not a fabricated success) when the store write
		// itself fails.
		let ccr = match &state.ccr {
			Some(ccr) => ccr,
			None => {
				return (
					StatusCode::SERVICE_UNAVAILABLE,
					Json(serde_json::json!({"error": "CCR not enabled"})),
				)
					.into_response();
			},
		};
		if !ccr_put(ccr, &hash, &content).await {
			return (
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(serde_json::json!({"error": "failed to store content in CCR backend"})),
			)
				.into_response();
		}
		state.ccr_created.fetch_add(1, Ordering::Relaxed);
		state.requests_compressed.fetch_add(1, Ordering::Relaxed);
		// See the JSON-body branch above: this endpoint's wire contract
		// IS the bare hash, so `hash.len()` is the correct subtractee.
		state
			.tokens_saved
			.fetch_add(original_size.saturating_sub(hash.len()) as u64, Ordering::Relaxed);

		let compressed_size = hash.len();
		Json(CcrCreateResponse {
			hash,
			token_savings_ratio:if original_size > 0 {
				original_size as f64 / compressed_size.max(1) as f64
			} else {
				1.0
			},
			original_size,
			compressed_size,
			marker_size:compressed_size,
		})
		.into_response()
	}
}

/// `GET /ccr/list` - reports entry count and backend kind for the active
/// CCR store (no listing of actual entries/hashes).
pub async fn handle_ccr_list(State(state):State<Arc<AppState>>) -> impl IntoResponse {
	match &state.ccr {
		Some(ccr) => {
			let entries = ccr_len(ccr).await;
			Json(serde_json::json!({
				"entries": entries,
				"backend": match state.mode {
					ProxyMode::Cache => "in_memory",
					ProxyMode::Token => "sqlite",
				},
				"mode": match state.mode {
					ProxyMode::Cache => "cache",
					ProxyMode::Token => "token",
				},
			}))
		},
		None => Json(serde_json::json!({"entries": 0, "message": "CCR not enabled"})),
	}
}

/// `DELETE /ccr/:hash` - removes a single entry from the CCR backend.
/// Returns 404 if the hash wasn't present, 503 if no backend is configured.
pub async fn handle_ccr_delete(
	State(state):State<Arc<AppState>>,
	axum::extract::Path(hash):axum::extract::Path<String>,
) -> impl IntoResponse {
	match &state.ccr {
		Some(ccr) => {
			let existed = ccr_del(ccr, &hash).await;
			if existed {
				(StatusCode::OK, Json(serde_json::json!({"deleted": true, "hash": hash})))
			} else {
				(
					StatusCode::NOT_FOUND,
					Json(serde_json::json!({"deleted": false, "hash": hash, "error": "not found"})),
				)
			}
		},
		None => {
			(
				StatusCode::SERVICE_UNAVAILABLE,
				Json(serde_json::json!({"error": "CCR not enabled"})),
			)
		},
	}
}

// ── Config reload ──────────────────────────────────────────────────

/// Hot-reload aphrodite.toml and apply compression config changes.
/// POST /reload - returns the newly loaded compression settings.
/// `POST /reload` - re-parse `aphrodite.toml` and apply its `[compression]`
/// thresholds to THIS listener's live `AppState` (report 07 F2/F4/T15) -
/// previously this endpoint parsed the file, echoed the values back, and
/// discarded them; a 200 response with `"reloaded": true` asserted a state
/// change that never happened. Other `[compression]` keys
/// (`engine_threshold_pct`, `catalog_mode`, `auto_expand*`) have no consumer
/// in this crate's proxy path (they're echoed for visibility, not applied -
/// see report 07 F8/F9 for their fate).
pub async fn handle_ccr_reload(State(state):State<Arc<AppState>>) -> impl IntoResponse {
	let config_path = std::env::var("APHRODITE_CONFIG_PATH").unwrap_or_else(|_| "aphrodite.toml".to_string());
	match crate::config::MultiConfig::load(&config_path) {
		Ok(config) => {
			let comp = config.compression.as_ref();
			let thresholds = resolve_thresholds(comp);
			state.cache_compress_threshold.store(thresholds.cache, Ordering::Relaxed);
			state.token_compress_threshold.store(thresholds.token, Ordering::Relaxed);
			state.inline_ccr_threshold.store(thresholds.inline, Ordering::Relaxed);
			state
				.code_multiplier_x100
				.store((thresholds.code_multiplier * 100.0) as u64, Ordering::Relaxed);
			let body = serde_json::json!({
				"reloaded": true,
				"applied": true,
				"config": config_path,
				"compression": {
					"tool_threshold_cache": thresholds.cache,
					"tool_threshold_token": thresholds.token,
					"inline_threshold": thresholds.inline,
					"code_multiplier": thresholds.code_multiplier,
				},
				// Parsed and visible, but not applied by this proxy (see doc
				// comment above).
				"parsed_only": {
					"auto_expand": comp.and_then(|c| c.auto_expand),
					"auto_expand_limit": comp.and_then(|c| c.auto_expand_limit),
					"terminal_threshold": comp.and_then(|c| c.terminal_threshold),
					"engine_threshold_pct": comp.and_then(|c| c.engine_threshold_pct),
					"catalog_mode": comp.and_then(|c| c.catalog_mode.clone()),
				}
			});
			tracing::info!(
				%config_path,
				cache_threshold = thresholds.cache,
				token_threshold = thresholds.token,
				inline_threshold = thresholds.inline,
				code_multiplier = thresholds.code_multiplier,
				"config reloaded - compression thresholds applied"
			);
			(StatusCode::OK, Json(body)).into_response()
		},
		Err(e) => {
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				Json(serde_json::json!({"error": format!("failed to reload: {e}")})),
			)
				.into_response()
		},
	}
}

// ── Health check ────────────────────────────────────────────────────

/// `GET /health` - local-only liveness check; does not call the upstream
/// API (see `/health/upstream` for that). Always returns 200 - capability
/// state (e.g. whether CCR is enabled) is conveyed via the JSON body
/// instead of the status code, since CCR is optional/opt-in.
pub async fn health_check(State(state):State<Arc<AppState>>) -> impl IntoResponse {
	let ccr_ok = state.ccr.is_some();

	(
		StatusCode::OK,
		Json(serde_json::json!({
			"status": "healthy",
			"ccr": ccr_ok,
			"mode": match state.mode {
				ProxyMode::Cache => "cache",
				ProxyMode::Token => "token",
			},
			"version": env!("CARGO_PKG_VERSION"),
			"fill_pct": state.fill_pct.load(Ordering::Relaxed) as f64 / 100.0,
		})),
	)
		.into_response()
}

// ── Tests ────────────────────────────────────────────────────────────

// `pub(crate)` (not private) so other in-crate test modules - e.g.
// `retrieve::tests` - can reach `test_state_with_ccr()` without duplicating
// `AppState`'s ~47-field literal (report 05 T5 verification).
#[cfg(test)]
pub(crate) mod tests {
	use super::*;

	#[test]
	fn test_compress_threshold_cache() {
		use std::{collections::HashMap, sync::Mutex};
		let state = AppState {
			client:HttpClient::new(),
			api_url:"https://upstream-openai.com".into(),
			model:"test".into(),
			api_key:"test".into(),
			ccr:None,
			add_markers:false,
			mode:ProxyMode::Cache,
			tool_relay:false,
			notify_url:None,
			notify_key:None,
			dev:false,
			requests_total:AtomicU64::new(0),
			requests_compressed:AtomicU64::new(0),
			tokens_saved:AtomicU64::new(0),
			ccr_hits:AtomicU64::new(0),
			ccr_misses:AtomicU64::new(0),
			ccr_created:AtomicU64::new(0),
			tool_relay_calls:AtomicU64::new(0),
			compression_ratio_ema:AtomicU64::new(200), // initial: 2.0x - conservative, avoids startup scale-up
			request_history:Mutex::new(VecDeque::new()),
			inline_ccr:Mutex::new(lru::LruCache::new(NonZeroUsize::new(1024).unwrap())),
			latency_buckets:[
				AtomicU64::new(0),
				AtomicU64::new(0),
				AtomicU64::new(0),
				AtomicU64::new(0),
				AtomicU64::new(0),
			],
			total_latency_micros:AtomicU64::new(0),
			last_errors:Mutex::new(VecDeque::new()),
			compressions_by_type:Mutex::new(HashMap::new()),
			response_cache:Mutex::new(lru::LruCache::new(NonZeroUsize::new(128).unwrap())),
			response_cache_ttl:std::time::Duration::from_secs(3600),
			cache_hits:AtomicU64::new(0),
			cache_misses:AtomicU64::new(0),
			fill_pct:AtomicU64::new(9000),
			task_tracker:TaskTracker::new(),
			inline_ccr_hits:AtomicU64::new(0),
			inline_ccr_misses:AtomicU64::new(0),
			tool_relay_success:AtomicU64::new(0),
			tool_relay_failure:AtomicU64::new(0),
			notify_success:AtomicU64::new(0),
			notify_failure:AtomicU64::new(0),
			upstream_errors_4xx:AtomicU64::new(0),
			upstream_errors_5xx:AtomicU64::new(0),
			upstream_timeouts:AtomicU64::new(0),
			upstream_connect_errors:AtomicU64::new(0),
			ccr_store_entries:AtomicU64::new(0),
			ccr_store_bytes:AtomicU64::new(0),
			request_body_bytes:AtomicU64::new(0),
			response_body_bytes:AtomicU64::new(0),
			upstream_latency_micros:AtomicU64::new(0),
			upstream_health_cache:std::sync::Mutex::new(None),
			cache_compress_threshold:AtomicUsize::new(CACHE_COMPRESS_THRESHOLD),
			token_compress_threshold:AtomicUsize::new(TOKEN_COMPRESS_THRESHOLD),
			inline_ccr_threshold:AtomicUsize::new(INLINE_CCR_THRESHOLD),
			code_multiplier_x100:AtomicU64::new(300),
		};
		assert_eq!(state.compress_threshold(), CACHE_COMPRESS_THRESHOLD);
	}

	#[test]
	fn test_compress_threshold_aphrodite() {
		let state = AppState { mode:ProxyMode::Token, ..test_state() };
		assert_eq!(state.compress_threshold(), TOKEN_COMPRESS_THRESHOLD);
	}

	// ── T15 (F2): TOML `[compression]` thresholds must actually be honored,
	// not silently discarded in favor of the compiled-in consts. ──
	#[test]
	fn test_resolve_thresholds_toml_overrides_defaults() {
		let comp = CompressionConfig {
			engine_threshold_pct:None,
			engine_protect_first:None,
			engine_protect_last:None,
			engine_min_msgs:None,
			tool_threshold_token:Some(512),
			tool_threshold_cache:Some(4096),
			terminal_threshold:None,
			inline_threshold:Some(2048),
			auto_expand:None,
			auto_expand_limit:None,
			catalog_mode:None,
			classifier_poll:None,
			code_multiplier:Some(5.0),
		};
		let t = resolve_thresholds(Some(&comp));
		assert_eq!(t.cache, 4096);
		assert_eq!(t.token, 512);
		assert_eq!(t.inline, 2048);
		assert_eq!(t.code_multiplier, 5.0);
	}

	#[test]
	fn test_resolve_thresholds_defaults_when_no_toml() {
		let t = resolve_thresholds(None);
		assert_eq!(t.cache, CACHE_COMPRESS_THRESHOLD);
		assert_eq!(t.token, TOKEN_COMPRESS_THRESHOLD);
		assert_eq!(t.inline, INLINE_CCR_THRESHOLD);
		// F10: the corrected default - 2 only ever existed because
		// "3.0".parse::<usize>() silently failed.
		assert_eq!(t.code_multiplier, 3.0);
	}

	// ── T15 (F4): `/reload` must actually apply the new thresholds to the
	// live `AppState`, not just re-parse and echo the file. ──
	#[test]
	fn test_handle_ccr_reload_applies_thresholds_to_state() {
		let dir = std::env::temp_dir();
		let path =
			dir.join(format!("aphrodite_reload_test_{}_{}.toml", std::process::id(), fnv1a_64(b"reload-test-salt")));
		std::fs::write(
			&path,
			r#"
[[proxies]]
name = "token"
mode = "token"

[compression]
tool_threshold_token = 999
tool_threshold_cache = 1234
inline_threshold = 77
code_multiplier = 6.5
"#,
		)
		.unwrap();

		// No other test in this crate reads/writes APHRODITE_CONFIG_PATH.
		std::env::set_var("APHRODITE_CONFIG_PATH", &path);
		let state = std::sync::Arc::new(test_state());
		let rt = tokio::runtime::Runtime::new().unwrap();
		let resp = rt.block_on(handle_ccr_reload(State(state.clone()))).into_response();
		std::env::remove_var("APHRODITE_CONFIG_PATH");
		let _ = std::fs::remove_file(&path);

		assert_eq!(resp.status(), axum::http::StatusCode::OK);
		assert_eq!(state.token_compress_threshold.load(Ordering::Relaxed), 999);
		assert_eq!(state.cache_compress_threshold.load(Ordering::Relaxed), 1234);
		assert_eq!(state.inline_ccr_threshold.load(Ordering::Relaxed), 77);
		assert_eq!(state.code_multiplier_x100.load(Ordering::Relaxed), 650);
	}

	#[test]
	fn test_stats_json_modes() {
		let cache = test_state();
		let stats = cache.stats_json();
		assert_eq!(stats["mode"], "cache");
		assert_eq!(stats["proxy"], "aphrodite");

		let mut aph = test_state();
		aph.mode = ProxyMode::Token;
		let stats = aph.stats_json();
		assert_eq!(stats["mode"], "token");
	}

	// ── T13 (F12): the enabled/disabled flag and the calls-stats object
	// must both be visible - a duplicate JSON key used to let the stats
	// object silently shadow the boolean. ──
	#[test]
	fn test_stats_json_tool_relay_enabled_flag_not_shadowed() {
		let mut state = test_state();
		state.tool_relay = true;
		let stats = state.stats_json();
		assert_eq!(stats["tool_relay_enabled"], true);
		assert!(
			stats["tool_relay"].is_object(),
			"the calls-stats object must still be present under its own key"
		);
		assert!(stats["tool_relay"]["total"].is_u64());
	}

	#[test]
	fn test_ccr_create_response_serde_shape() {
		// Pins the wire contract of POST /ccr/create: field names and values
		// as they actually serialize, not just struct-literal field access.
		let resp = CcrCreateResponse {
			hash:"abc123".into(),
			token_savings_ratio:2.5,
			original_size:100,
			compressed_size:40,
			marker_size:40,
		};
		let v = serde_json::to_value(&resp).unwrap();
		assert_eq!(v["hash"], "abc123");
		assert_eq!(v["original_size"], 100);
		assert_eq!(v["compressed_size"], 40);
		assert_eq!(v["marker_size"], 40);
		assert!((v["token_savings_ratio"].as_f64().unwrap() - 2.5).abs() < 0.01);
		// Regression guard for bench_01/02 (F3): the field is
		// `token_savings_ratio`, never `compression_ratio`.
		assert!(v.get("compression_ratio").is_none());
	}

	#[test]
	fn test_tool_relay_response_sync_serde_shape() {
		let resp = ToolRelayResponse {
			success:true,
			result:Some(serde_json::json!({"found": true})),
			error:None,
			async_call:false,
		};
		let v = serde_json::to_value(&resp).unwrap();
		assert_eq!(v["success"], true);
		assert_eq!(v["async_call"], false);
		assert_eq!(v["result"]["found"], true);
		assert!(v["error"].is_null());
	}

	#[test]
	fn test_tool_relay_response_async_serde_shape() {
		let resp = ToolRelayResponse { success:true, result:None, error:None, async_call:true };
		let v = serde_json::to_value(&resp).unwrap();
		assert_eq!(v["async_call"], true);
		assert!(v["result"].is_null());
	}

	// ── T3: detect_content_type ─────────────────────────────────
	#[test]
	fn test_detect_content_type_json_tool_output() {
		assert_eq!(detect_content_type(r#"{"exit_code": 0, "output": "ok"}"#), "tool_output");
	}

	#[test]
	fn test_detect_content_type_invalid_json_is_text() {
		// Starts with '{' but isn't valid JSON - must not be misclassified.
		assert_eq!(detect_content_type("{ not json at all"), "text");
	}

	#[test]
	fn test_detect_content_type_json_array() {
		assert_eq!(detect_content_type(r#"[{"a":1},{"a":2}]"#), "json");
	}

	#[test]
	fn test_detect_content_type_rust_code() {
		let src = "use std::fmt;\nfn add(a:i32, b:i32) -> i32 {\n    a + b\n}\n";
		assert_eq!(detect_content_type(src), "code_rust");
	}

	#[test]
	fn test_detect_content_type_python_code() {
		let src = "import os\nclass Foo:\n    def bar(self):\n        pass\n";
		assert_eq!(detect_content_type(src), "code_python");
	}

	#[test]
	fn test_detect_content_type_go_code() {
		let src = "package main\nimport (\n\t\"fmt\"\n)\nfunc main() {\n\tfmt.Println(\"hi\")\n}\n";
		assert_eq!(detect_content_type(src), "code_go");
	}

	#[test]
	fn test_detect_content_type_js_code() {
		let src = "import { foo } from 'bar';\nexport const add = (a, b) => a + b;\nconst x = 1;\nconst y = 2;\n";
		assert_eq!(detect_content_type(src), "code_js");
	}

	#[test]
	fn test_detect_content_type_error_first_line() {
		assert_eq!(
			detect_content_type("Traceback (most recent call last):\n  File \"x.py\", line 1\nValueError: bad\n"),
			"error"
		);
	}

	#[test]
	fn test_detect_content_type_diff() {
		let d = "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,3 +1,4 @@\n+added a \
		         line\n";
		assert_eq!(detect_content_type(d), "diff");
	}

	#[test]
	fn test_detect_content_type_log_lines() {
		// Must not start with '{'/'[' (that short-circuits to the JSON branch).
		let log = "starting up\n[INFO] service ready\n[WARN] disk low\n[ERROR] connection lost\n";
		assert_eq!(detect_content_type(log), "log");
	}

	#[test]
	fn test_detect_content_type_empty_is_text() {
		assert_eq!(detect_content_type(""), "text");
	}

	#[test]
	fn test_detect_content_type_plain_text() {
		assert_eq!(detect_content_type("just some plain text\nnothing special\n"), "text");
	}

	// ── T3: generate_metadata ───────────────────────────────────
	#[test]
	fn test_generate_metadata_rust_has_lang_and_fns() {
		let src = "fn add(a:i32, b:i32) -> i32 {\n    a + b\n}\n";
		let meta = generate_metadata(src, "code_rust");
		assert!(meta.contains("lang=rs"));
		assert!(meta.contains("fns=add"));
	}

	#[test]
	fn test_generate_metadata_escapes_pipes() {
		// The 'On branch' line can legally contain a literal '|' - must be escaped to
		// '/'.
		let src = "On branch feature|weird\n";
		let meta = generate_metadata(src, "git");
		assert!(!meta.contains('|'), "metadata must not contain a raw pipe: {meta}");
	}

	#[test]
	fn test_generate_metadata_max_400_chars() {
		let src = (0..100).map(|i| format!("fn f{i}() {{}}")).collect::<Vec<_>>().join("\n");
		let meta = generate_metadata(&src, "code_rust");
		assert!(meta.chars().count() <= 400, "metadata too long: {} chars", meta.chars().count());
	}

	#[test]
	fn test_generate_metadata_error_branch_no_panic_on_multibyte_utf8() {
		// Multi-byte UTF-8 characters sit right around the ".rs:" match position -
		// byte-index slicing here must not panic on a non-char-boundary.
		let src = "日本語エラー at src/日本.rs:10:5 something\n";
		let meta = generate_metadata(src, "error");
		// No assertion beyond "did not panic" is required, but sanity-check shape.
		assert!(meta.is_empty() || meta.contains("trace=") || meta.contains("msg="));
	}

	#[test]
	fn test_generate_metadata_text_has_line_count() {
		let meta = generate_metadata("a\nb\nc\n", "text");
		assert_eq!(meta, "ln=3");
	}

	// ── T3: build_preview ────────────────────────────────────────
	#[test]
	fn test_build_preview_code_has_ct_prefix() {
		let src = "fn add(a:i32, b:i32) -> i32 {\n    a + b\n}\n";
		let preview = build_preview(src, "code_rust");
		assert!(preview.starts_with("[code_rust:"));
	}

	#[test]
	fn test_build_preview_error_has_ct_prefix_via_error_line() {
		let src = "some noise\nerror[E0308]: mismatched types\nmore noise\n";
		let preview = build_preview(src, "error");
		assert!(preview.contains("error[E0308]"));
	}

	#[test]
	fn test_build_preview_diff_has_ct_prefix() {
		let src = "diff --git a/x b/x\n--- a/x\n+++ a/x\n";
		let preview = build_preview(src, "diff");
		assert!(preview.starts_with("diff --git"));
	}

	#[test]
	fn test_build_preview_json_has_ct_prefix() {
		let src = "{\"a\":1,\"b\":2}\n";
		let preview = build_preview(src, "json");
		assert!(preview.contains("keys"));
	}

	// ── T3: cache_key_from_body / fnv1a_64 ────────────────────────
	#[test]
	fn test_cache_key_from_body_deterministic() {
		let body = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#;
		let k1 = cache_key_from_body(body, "key-a");
		let k2 = cache_key_from_body(body, "key-a");
		assert!(k1.is_some());
		assert_eq!(k1, k2);
	}

	#[test]
	fn test_cache_key_from_body_differs_by_api_key() {
		let body = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#;
		let k1 = cache_key_from_body(body, "key-a");
		let k2 = cache_key_from_body(body, "key-b");
		assert_ne!(k1, k2);
	}

	#[test]
	fn test_cache_key_from_body_none_on_junk() {
		assert_eq!(cache_key_from_body(b"not json", "key"), None);
		assert_eq!(cache_key_from_body(b"{}", "key"), None); // missing model/messages
	}

	// ── T8 (F3): the cache key must cover the request parameters that
	// change what a valid response can look like, and streamed requests
	// must never be cached at all. ──
	#[test]
	fn test_cache_key_from_body_differs_by_tools_and_temperature_and_stream() {
		let base = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}]}"#;
		let with_tools =
			br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"tools":[{"type":"function"}]}"#;
		let with_temp = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"temperature":0.7}"#;
		let k_base = cache_key_from_body(base, "key");
		let k_tools = cache_key_from_body(with_tools, "key");
		let k_temp = cache_key_from_body(with_temp, "key");
		assert!(k_base.is_some());
		assert_ne!(k_base, k_tools, "differing `tools` must produce a different cache key");
		assert_ne!(k_base, k_temp, "differing `temperature` must produce a different cache key");
		assert_ne!(k_tools, k_temp);
	}

	#[test]
	fn test_cache_key_from_body_none_when_streaming() {
		let streamed = br#"{"model":"gpt-4","messages":[{"role":"user","content":"hi"}],"stream":true}"#;
		assert_eq!(cache_key_from_body(streamed, "key"), None);
	}

	// ── T1 remainder (F5, report 06): response_cache entries must expire
	// on their own instead of being replayed forever. ──
	#[test]
	fn test_response_cache_get_expires_past_ttl() {
		let mut state = test_state();
		state.response_cache_ttl = std::time::Duration::from_millis(1);
		state.response_cache.lock().unwrap().put(42, (std::time::Instant::now(), b"cached".to_vec()));
		std::thread::sleep(std::time::Duration::from_millis(20));
		assert_eq!(response_cache_get(&state, 42), None, "expired entry must not be returned");
		assert!(
			state.response_cache.lock().unwrap().peek(&42).is_none(),
			"expired entry must be evicted, not just skipped"
		);
	}

	#[test]
	fn test_response_cache_get_hits_within_ttl() {
		let mut state = test_state();
		state.response_cache_ttl = std::time::Duration::from_secs(3600);
		state.response_cache.lock().unwrap().put(7, (std::time::Instant::now(), b"cached".to_vec()));
		assert_eq!(response_cache_get(&state, 7), Some(b"cached".to_vec()));
	}

	#[test]
	fn test_fnv1a_64_known_vectors() {
		// FNV-1a 64-bit offset basis is the hash of the empty input.
		assert_eq!(fnv1a_64(b""), 14695981039346656037);
		// Two different inputs should (overwhelmingly likely) hash differently.
		assert_ne!(fnv1a_64(b"a"), fnv1a_64(b"b"));
	}

	fn test_state() -> AppState {
		use std::{collections::HashMap, sync::Mutex};
		AppState {
			client:HttpClient::new(),
			api_url:"https://upstream-openai.com".into(),
			model:"default-model".into(),
			api_key:"test".into(),
			ccr:None,
			add_markers:false,
			mode:ProxyMode::Cache,
			tool_relay:false,
			notify_url:None,
			notify_key:None,
			dev:false,
			requests_total:AtomicU64::new(0),
			requests_compressed:AtomicU64::new(0),
			tokens_saved:AtomicU64::new(0),
			ccr_hits:AtomicU64::new(0),
			ccr_misses:AtomicU64::new(0),
			ccr_created:AtomicU64::new(0),
			tool_relay_calls:AtomicU64::new(0),
			compression_ratio_ema:AtomicU64::new(200), // initial: 2.0x - conservative, avoids startup scale-up
			request_history:Mutex::new(VecDeque::new()),
			inline_ccr:Mutex::new(lru::LruCache::new(NonZeroUsize::new(1024).unwrap())),
			latency_buckets:[
				AtomicU64::new(0),
				AtomicU64::new(0),
				AtomicU64::new(0),
				AtomicU64::new(0),
				AtomicU64::new(0),
			],
			total_latency_micros:AtomicU64::new(0),
			last_errors:Mutex::new(VecDeque::new()),
			compressions_by_type:Mutex::new(HashMap::new()),
			response_cache:Mutex::new(lru::LruCache::new(NonZeroUsize::new(128).unwrap())),
			response_cache_ttl:std::time::Duration::from_secs(3600),
			cache_hits:AtomicU64::new(0),
			cache_misses:AtomicU64::new(0),
			fill_pct:AtomicU64::new(9000),
			task_tracker:TaskTracker::new(),
			inline_ccr_hits:AtomicU64::new(0),
			inline_ccr_misses:AtomicU64::new(0),
			tool_relay_success:AtomicU64::new(0),
			tool_relay_failure:AtomicU64::new(0),
			notify_success:AtomicU64::new(0),
			notify_failure:AtomicU64::new(0),
			upstream_errors_4xx:AtomicU64::new(0),
			upstream_errors_5xx:AtomicU64::new(0),
			upstream_timeouts:AtomicU64::new(0),
			upstream_connect_errors:AtomicU64::new(0),
			ccr_store_entries:AtomicU64::new(0),
			ccr_store_bytes:AtomicU64::new(0),
			request_body_bytes:AtomicU64::new(0),
			response_body_bytes:AtomicU64::new(0),
			upstream_latency_micros:AtomicU64::new(0),
			upstream_health_cache:std::sync::Mutex::new(None),
			cache_compress_threshold:AtomicUsize::new(CACHE_COMPRESS_THRESHOLD),
			token_compress_threshold:AtomicUsize::new(TOKEN_COMPRESS_THRESHOLD),
			inline_ccr_threshold:AtomicUsize::new(INLINE_CCR_THRESHOLD),
			code_multiplier_x100:AtomicU64::new(300),
		}
	}

	/// `test_state()` with a real (in-memory) CCR backend attached, since
	/// `compress_chat_completion` only compresses when `state.ccr` is `Some`.
	///
	/// `pub(crate)` (rather than private) so other in-crate test modules -
	/// e.g. `retrieve::tests` - can build a real `AppState` without
	/// duplicating this ~40-field literal (report 05 T5 verification).
	pub(crate) fn test_state_with_ccr() -> AppState {
		AppState {
			ccr:Some(std::sync::Arc::new(InMemoryCcrStore::with_capacity_and_ttl(
				1000,
				std::time::Duration::from_secs(300),
			))),
			mode:ProxyMode::Token,
			..test_state()
		}
	}

	/// A `CcrStore` whose `put` always fails - used to test report 02's F4
	/// fix: a failed store write must never be followed by replacing
	/// content with an unresolvable marker.
	struct FailingCcrStore;
	impl headroom_core::ccr::CcrStore for FailingCcrStore {
		fn put(&self, _hash:&str, _payload:&str) -> bool { false }
		fn get(&self, _hash:&str) -> Option<String> { None }
		fn len(&self) -> usize { 0 }
		fn del(&self, _hash:&str) -> bool { false }
	}

	fn test_state_with_failing_ccr() -> AppState {
		AppState {
			ccr:Some(std::sync::Arc::new(FailingCcrStore)),
			mode:ProxyMode::Token,
			..test_state()
		}
	}

	// ── T4 (F4): a failed ccr_put must leave content uncompressed, not
	// silently swapped for an unresolvable marker. ──
	#[test]
	fn test_compress_chat_completion_ccr_put_failure_leaves_content_uncompressed() {
		let state = test_state_with_failing_ccr();
		let content = "fn answer() -> i32 { 42 }\n".repeat(200); // well above threshold
		let body = chat_completion_body(&content);
		let rt = tokio::runtime::Runtime::new().unwrap();
		let result = rt.block_on(compress_chat_completion(&state, &body, None));
		// Nothing could be safely compressed (the only candidate's store
		// write failed), so the whole response must come back `None`
		// (unmodified pass-through), not `Some` with a poisoned marker.
		assert!(result.is_none(), "a failed ccr_put must not produce a compressed response");
	}

	// ── T5 (F14): /ccr/create must report unavailability, not a
	// fabricated success, when CCR isn't enabled or the store write fails. ──
	#[test]
	fn test_handle_ccr_create_503_when_ccr_disabled() {
		let mut state = test_state();
		state.ccr = None;
		let state = std::sync::Arc::new(state);
		let rt = tokio::runtime::Runtime::new().unwrap();
		let resp = rt
			.block_on(handle_ccr_create(
				State(state),
				axum::http::HeaderMap::new(),
				Bytes::from_static(b"hello world"),
			))
			.into_response();
		assert_eq!(resp.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
	}

	#[test]
	fn test_handle_ccr_create_500_when_put_fails() {
		let state = std::sync::Arc::new(test_state_with_failing_ccr());
		let rt = tokio::runtime::Runtime::new().unwrap();
		let resp = rt
			.block_on(handle_ccr_create(
				State(state),
				axum::http::HeaderMap::new(),
				Bytes::from_static(b"hello world"),
			))
			.into_response();
		assert_eq!(resp.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
	}

	fn chat_completion_body(content:&str) -> Vec<u8> {
		serde_json::json!({
			"choices": [{
				"message": {"role": "assistant", "content": content}
			}]
		})
		.to_string()
		.into_bytes()
	}

	// ── T4: compress_chat_completion ────────────────────────────
	#[test]
	fn test_compress_chat_completion_above_threshold_produces_marker() {
		// Token mode base threshold is 1024B; a large repeated text block
		// stays well above any auto-tuned multiplier of it.
		let content = "the quick brown fox jumps over the lazy dog. ".repeat(200);
		let body = chat_completion_body(&content);
		let state = test_state_with_ccr();

		let result = tokio::runtime::Runtime::new()
			.unwrap()
			.block_on(compress_chat_completion(&state, &body, None));

		let response = result.expect("content above threshold must be compressed");
		let new_content = response["choices"][0]["message"]["content"].as_str().unwrap();
		assert!(new_content.contains("<<<CCR:"), "expected a CCR marker, got: {new_content}");

		// The original content must be retrievable from the CCR store.
		let ccr = state.ccr.as_ref().unwrap().clone();
		let hash = compute_key(content.as_bytes());
		let stored = tokio::runtime::Runtime::new().unwrap().block_on(ccr_get(&ccr, &hash));
		assert_eq!(stored.as_deref(), Some(content.as_str()));
	}

	#[test]
	fn test_compress_chat_completion_below_threshold_is_none() {
		let content = "short reply";
		let body = chat_completion_body(content);
		let state = test_state_with_ccr();

		let result = tokio::runtime::Runtime::new()
			.unwrap()
			.block_on(compress_chat_completion(&state, &body, None));

		assert!(result.is_none(), "short content must not be compressed: {result:?}");
	}

	#[test]
	fn test_compress_chat_completion_tool_call_arguments_compressed_independently() {
		let big_args = serde_json::json!({"data": "x".repeat(4000)}).to_string();
		let body = serde_json::json!({
			"choices": [{
				"message": {
					"role": "assistant",
					"content": "ok",
					"tool_calls": [{
						"function": {"name": "f", "arguments": big_args}
					}]
				}
			}]
		})
		.to_string()
		.into_bytes();
		let state = test_state_with_ccr();

		let result = tokio::runtime::Runtime::new()
			.unwrap()
			.block_on(compress_chat_completion(&state, &body, None));

		let response = result.expect("large tool-call arguments must be compressed");
		// Message content ("ok") is short and must remain untouched.
		assert_eq!(response["choices"][0]["message"]["content"], "ok");
		let new_args = response["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"]
			.as_str()
			.unwrap();
		assert!(
			new_args.contains("<<<CCR:"),
			"expected tool-call arguments to be compressed: {new_args}"
		);
	}

	#[test]
	fn test_compress_chat_completion_budget_header_lowers_effective_threshold() {
		// A large-but-moderate payload: compressed when the budget header
		// requests aggressive compression (fill=0 -> 0.5x multiplier), but
		// left alone with no budget header (multiplier 1.0x) if it sits
		// between the two thresholds.
		let content = "line of moderate length text content here.\n".repeat(40); // ~1760B
		let body = chat_completion_body(&content);

		let state_no_budget = test_state_with_ccr();
		let result_no_budget =
			tokio::runtime::Runtime::new()
				.unwrap()
				.block_on(compress_chat_completion(&state_no_budget, &body, None));

		let state_low_budget = test_state_with_ccr();
		let result_low_budget = tokio::runtime::Runtime::new().unwrap().block_on(compress_chat_completion(
			&state_low_budget,
			&body,
			Some("0"),
		));

		// A lower budget must never compress *less* than no budget at all.
		if result_no_budget.is_some() {
			assert!(
				result_low_budget.is_some(),
				"lower budget must compress at least as much as no budget"
			);
		}
	}

	// ── T5: execute_tool_relay ───────────────────────────────────
	#[test]
	fn test_execute_tool_relay_retrieve_missing_hash_param() {
		let state = test_state_with_ccr();
		let result = tokio::runtime::Runtime::new().unwrap().block_on(execute_tool_relay(
			&state,
			"aphrodite_retrieve",
			&serde_json::json!({}),
		));
		assert_eq!(result, Err("missing hash".to_string()));
	}

	#[test]
	fn test_execute_tool_relay_unknown_tool_is_err() {
		let state = test_state_with_ccr();
		let result = tokio::runtime::Runtime::new().unwrap().block_on(execute_tool_relay(
			&state,
			"not_a_real_tool",
			&serde_json::json!({}),
		));
		assert!(result.is_err());
	}

	#[test]
	fn test_execute_tool_relay_compress_small_content_stores_inline_and_returns_marker() {
		let state = test_state_with_ccr();
		let content = "tiny"; // well under INLINE_CCR_THRESHOLD (256B)
		let result = tokio::runtime::Runtime::new().unwrap().block_on(execute_tool_relay(
			&state,
			"aphrodite_compress",
			&serde_json::json!({"content": content}),
		));
		let v = result.expect("compress must succeed");
		assert!(v["compressed"].as_str().unwrap().contains("<<<CCR:"));
		assert_eq!(v["original_size"], content.len());
	}

	// ── T7 (F5, report 06): tiny relay-compress content must also land in
	// the durable backend, not only the 1024-entry inline LRU - otherwise a
	// busy session evicts the "durable-looking" marker's only copy within
	// minutes. Verifies the durable store has the content directly,
	// independent of the inline cache. ──
	#[test]
	fn test_execute_tool_relay_compress_small_content_also_stores_durably() {
		let state = test_state_with_ccr();
		let content = "tiny"; // well under INLINE_CCR_THRESHOLD (256B)
		let rt = tokio::runtime::Runtime::new().unwrap();
		let result = rt.block_on(execute_tool_relay(&state, "aphrodite_compress", &serde_json::json!({"content": content})));
		let v = result.expect("compress must succeed");
		let hash = v["hash"].as_str().unwrap().to_string();

		// Simulate the inline entry having been evicted (busy session /
		// restart) by going straight to the durable backend.
		let ccr = state.ccr.as_ref().unwrap();
		let durable = rt.block_on(ccr_get(ccr, &hash));
		assert_eq!(durable.as_deref(), Some(content), "tiny content must also be durable, not inline-only");
	}

	#[test]
	fn test_execute_tool_relay_retrieve_finds_inline_entry() {
		let state = test_state_with_ccr();
		let content = "tiny";
		let compressed = tokio::runtime::Runtime::new()
			.unwrap()
			.block_on(execute_tool_relay(
				&state,
				"aphrodite_compress",
				&serde_json::json!({"content": content}),
			))
			.unwrap();
		let hash = compressed["hash"].as_str().unwrap().to_string();

		let retrieved = tokio::runtime::Runtime::new()
			.unwrap()
			.block_on(execute_tool_relay(
				&state,
				"aphrodite_retrieve",
				&serde_json::json!({"hash": hash}),
			))
			.unwrap();
		assert_eq!(retrieved["found"], true);
		assert_eq!(retrieved["content"], content);
	}

	// ── T5 (F3): execute_tool_relay's "aphrodite_retrieve" arm must
	// normalize the hash argument the same way `resolve_one` already does -
	// strip a `|type|size` marker-body suffix an LLM might echo back, and
	// trim surrounding whitespace.
	#[test]
	fn test_execute_tool_relay_retrieve_normalizes_pipe_suffixed_and_whitespace_hash() {
		let state = test_state_with_ccr();
		let content = "tiny";
		let rt = tokio::runtime::Runtime::new().unwrap();
		let compressed = rt
			.block_on(execute_tool_relay(
				&state,
				"aphrodite_compress",
				&serde_json::json!({"content": content}),
			))
			.unwrap();
		let hash = compressed["hash"].as_str().unwrap().to_string();

		for hash_arg in [hash.clone(), format!("{hash}|tool|1024"), format!("  {hash}  ")] {
			let retrieved = rt
				.block_on(execute_tool_relay(
					&state,
					"aphrodite_retrieve",
					&serde_json::json!({"hash": hash_arg}),
				))
				.unwrap();
			assert_eq!(retrieved["found"], true, "hash arg {hash_arg:?} must resolve: {retrieved:?}");
			assert_eq!(retrieved["content"], content);
		}
	}

	// ── T15 (F2): regression tests for the historical corpus examples ────
	// These exercise the real Rust code (unlike Maintain/examples/*.py,
	// which re-implement the buggy/fixed logic in Python and can never
	// catch a Rust regression - see Maintain/examples/README.md).

	/// Corpus 07_tokens_saved.py: the AtomicU64 must actually be incremented
	/// on the real compression path, not just exist unused in /stats.
	#[test]
	fn regression_07_tokens_saved_increments_on_compress() {
		let content = "the quick brown fox jumps over the lazy dog. ".repeat(200);
		let body = chat_completion_body(&content);
		let state = test_state_with_ccr();

		assert_eq!(state.tokens_saved.load(Ordering::Relaxed), 0);
		let result = tokio::runtime::Runtime::new()
			.unwrap()
			.block_on(compress_chat_completion(&state, &body, None));
		assert!(result.is_some(), "content above threshold must compress");
		assert!(
			state.tokens_saved.load(Ordering::Relaxed) > 0,
			"tokens_saved must be incremented by the real compression path"
		);
	}

	/// Corpus 11_should_compress.py: threshold gating must actually skip
	/// compression below threshold, not compress unconditionally.
	#[test]
	fn regression_11_below_threshold_skips_compression_and_counter() {
		let content = "short reply below any threshold";
		let body = chat_completion_body(content);
		let state = test_state_with_ccr();

		let result = tokio::runtime::Runtime::new()
			.unwrap()
			.block_on(compress_chat_completion(&state, &body, None));
		assert!(result.is_none(), "below-threshold content must not compress");
		assert_eq!(
			state.tokens_saved.load(Ordering::Relaxed),
			0,
			"no savings should be recorded when nothing was compressed"
		);
	}

	/// Corpus 13_engine_truncation.py: a CCR marker must never be truncated
	/// mid-terminator by a preview-length budget - format_ccr_output's
	/// output must always contain a complete `<<<CCR:hash|type|size>>>` line,
	/// regardless of how long the preview or metadata are.
	#[test]
	fn regression_13_marker_terminator_never_truncated() {
		let hash = "abc123def456abc123def456abc123def456";
		let huge_preview = "x".repeat(10_000);
		let huge_metadata = "y".repeat(10_000);
		let out = format_ccr_output(&huge_preview, "text", &huge_metadata, None, hash, 123456);
		let expected_terminator = format!("<<<CCR:{hash}|text|123456>>>");
		assert!(
			out.contains(&expected_terminator),
			"marker terminator must always be complete and unsliced, regardless of preview/metadata length"
		);
	}

	// ── T8: property tests ───────────────────────────────────────
	use proptest::{prop_assert, proptest};

	proptest! {
		/// `detect_content_type` and `generate_metadata` must never panic on
		/// arbitrary UTF-8 input (this is literally the module's threat
		/// model - arbitrary tool output), and the generated metadata must
		/// respect its own documented invariants: no pipe/newline, <=400 chars.
		#[test]
		fn prop_classifier_and_metadata_never_panic(s in ".*") {
			let ct = detect_content_type(&s);
			let meta = generate_metadata(&s, ct);
			prop_assert!(!meta.contains('|'));
			prop_assert!(!meta.contains('\n'));
			prop_assert!(meta.chars().count() <= 400);
		}
	}
}
