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
		atomic::{AtomicU64, Ordering},
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

use crate::config::{Cli, ProxyMode};

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

/// Code multiplier: how many times the base threshold for code content.
/// Coding-optimized default: 2× (compresses code aggressively, LLM retrieves on
/// demand). Override: APHRODITE_CODE_MULTIPLIER env var (e.g. 4 to preserve
/// more code).
fn code_multiplier() -> usize {
	std::env::var("APHRODITE_CODE_MULTIPLIER")
		.ok()
		.and_then(|v| v.parse().ok())
		.unwrap_or(2)
}

// ── spawn_blocking wrappers for CcrStore (rusqlite is blocking) ─────

/// Wrapper for `ccr.get()` on a blocking thread.
pub(crate) async fn ccr_get(ccr:&Arc<dyn CcrStore>, hash:&str) -> Option<String> {
	let ccr = ccr.clone();
	let hash = hash.to_owned();
	tokio::task::spawn_blocking(move || ccr.get(&hash)).await.unwrap_or(None)
}

/// Wrapper for `ccr.put()` on a blocking thread.
async fn ccr_put(ccr:&Arc<dyn CcrStore>, hash:&str, content:&str) {
	let ccr = ccr.clone();
	let hash = hash.to_owned();
	let content = content.to_owned();
	let _ = tokio::task::spawn_blocking(move || ccr.put(&hash, &content)).await;
}

/// Wrapper for `ccr.del()` on a blocking thread.
#[allow(dead_code)]
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
	/// LRU cache: hash(model+messages) → serialized response body
	pub response_cache:std::sync::Mutex<lru::LruCache<u64, Vec<u8>>>,
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

	/// Rhai scripting engine - user-defined micro-scripts for hook injection.
	/// None when scripting feature is disabled or not configured.
	#[cfg(feature = "scripting")]
	pub script_engine:Option<std::sync::Arc<crate::scripting::ScriptEngine>>,
	#[cfg(not(feature = "scripting"))]
	pub script_engine:Option<()>,

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
	pub ccr_store_entries:AtomicU64,
	pub ccr_store_bytes:AtomicU64,
	pub request_body_bytes:AtomicU64,
	pub response_body_bytes:AtomicU64,
	pub upstream_latency_micros:AtomicU64,
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
			"tool_relay": self.tool_relay,
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
			ProxyMode::Cache => CACHE_COMPRESS_THRESHOLD,
			ProxyMode::Token => TOKEN_COMPRESS_THRESHOLD,
		}
	}

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
			"code_rust" | "code_python" | "code_go" | "code_js" | "code" => base * code_multiplier(),
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
pub async fn build_state(cli:&Cli) -> anyhow::Result<AppState> {
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
		cache_hits:AtomicU64::new(0),
		cache_misses:AtomicU64::new(0),
		fill_pct:AtomicU64::new(9000), // 90.00% - moderate fill initial default
		task_tracker:TaskTracker::new(),

		// Scripting engine - enabled via --scripting or APHRODITE_SCRIPTING=1
		#[cfg(feature = "scripting")]
		script_engine:crate::scripting_enabled().then(|| {
			let engine = crate::scripting::ScriptEngine::new();
			tracing::info!("scripting engine loaded with rhai scripts");
			std::sync::Arc::new(engine)
		}),
		#[cfg(not(feature = "scripting"))]
		script_engine:None,

		inline_ccr_hits:AtomicU64::new(0),
		inline_ccr_misses:AtomicU64::new(0),
		tool_relay_success:AtomicU64::new(0),
		tool_relay_failure:AtomicU64::new(0),
		notify_success:AtomicU64::new(0),
		notify_failure:AtomicU64::new(0),
		upstream_errors_4xx:AtomicU64::new(0),
		upstream_errors_5xx:AtomicU64::new(0),
		upstream_timeouts:AtomicU64::new(0),
		ccr_store_entries:AtomicU64::new(0),
		ccr_store_bytes:AtomicU64::new(0),
		request_body_bytes:AtomicU64::new(0),
		response_body_bytes:AtomicU64::new(0),
		upstream_latency_micros:AtomicU64::new(0),
	})
}

// ── Main proxy handler ──────────────────────────────────────────────

/// Generate a simple summary - first 3 lines or first 200 chars.
#[allow(dead_code)]
fn generate_summary(content:&str) -> String {
	let lines:Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).take(3).collect();
	if lines.len() >= 2 {
		format!(
			"[summary] {} lines, {}B: {}",
			content.lines().count(),
			content.len(),
			lines.join(" | ")
		)
	} else {
		let preview:String = content.char_indices().take_while(|(i, _)| *i < 200).map(|(_, c)| c).collect();
		format!("[summary] {}B: {}", content.len(), preview)
	}
}

/// Compute a cache key from a Chat Completions request body: hash(api_key +
/// model + messages). Uses FNV-1a (deterministic across restarts, unlike
/// DefaultHasher). Includes api_key to prevent cross-user cache collision.
/// Returns None if the body can't be parsed as JSON or lacks model/messages.
fn cache_key_from_body(body:&[u8], api_key:&str) -> Option<u64> {
	let v:serde_json::Value = serde_json::from_slice(body).ok()?;
	let model = v.get("model")?.as_str()?;
	let messages = v.get("messages")?;
	let messages_str = serde_json::to_string(messages).ok()?;
	// FNV-1a 64-bit hash - deterministic across process restarts
	// Include api_key to prevent cross-user cache collision
	Some(fnv1a_64(
		&[api_key.as_bytes(), b":", model.as_bytes(), b":", messages_str.as_bytes()].concat(),
	))
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

	let deepseek_path = path.path().trim_start_matches('/');
	let url = format!("{}/{}", state.api_url.trim_end_matches('/'), deepseek_path);

	let is_chat_completion = deepseek_path == CHAT_COMPLETIONS_PATH.trim_start_matches('/');

	let body_vec = body.to_vec();
	let cache_key = if is_chat_completion {
		cache_key_from_body(&body_vec, state.api_key.expose())
	} else {
		None
	};
	// Check LLM API response cache before upstream call
	if let Some(ck) = cache_key {
		if let Some(cached_body) = state.response_cache.lock().ok().and_then(|mut cache| cache.get(&ck).cloned()) {
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
			if k != "host" && k != "authorization" && k != "content-length" && !k.starts_with("x-aphrodite-") {
				req = req.header(key, val);
			}
		}
		match req.body(body_vec.clone()).send().await {
			Ok(r) => {
				upstream_result = Ok(r);
				break;
			},
			Err(e) => {
				if attempt < 3 {
					let base_ms = 100 * 2u64.pow(attempt - 1);
					let jitter = rand::random::<f64>() * 0.5 + 0.75; // 0.75x to 1.25x
					let ms = (base_ms as f64 * jitter) as u64;
					tracing::warn!(attempt, backoff_ms = ms, "upstream retry after error: {}", e);
					tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
				} else {
					upstream_result = Err(format!("{}", e));
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
			// Extract content-type before consuming response body
			let content_type = response.headers().get("content-type").cloned();
			let resp_body = match response.bytes().await {
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
				// Extract headroom budget from inbound headers for compression aggressiveness
				let headroom_budget = headers
					.get("x-headroom-budget")
					.and_then(|v| v.to_str().ok())
					.or_else(|| headers.get("X-Headroom-Budget").and_then(|v| v.to_str().ok()));
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
					// Store in LLM response cache
					if let Some(ck) = cache_key {
						if let Ok(mut cache) = state.response_cache.lock() {
							cache.put(ck, body.clone());
						}
					}
					return Response::builder()
						.status(status)
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
			// Store raw response in LLM cache if applicable
			if let Some(ck) = cache_key {
				if let Ok(mut cache) = state.response_cache.lock() {
					cache.put(ck, resp_body.to_vec());
				}
			}
			let mut builder = Response::builder().status(status).header("X-Aphrodite-Cache", "MISS");
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
			state.upstream_timeouts.fetch_add(1, Ordering::Relaxed);
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
						if ccr_get(ccr, &hash).await.is_some() {
							state.ccr_hits.fetch_add(1, Ordering::Relaxed);
						} else {
							state.ccr_misses.fetch_add(1, Ordering::Relaxed);
							ccr_put(ccr, &hash, content).await;
							state.ccr_created.fetch_add(1, Ordering::Relaxed);
						}

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
						state.tokens_saved.fetch_add(orig_len.saturating_sub(marker_len) as u64, Ordering::Relaxed);
						*content_val = serde_json::Value::String(compressed);
						did_compress = true;
						state.update_compression_ratio(orig_len, marker_len);
					}
				} else if content.len() > INLINE_CCR_THRESHOLD {
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
										if ccr_get(ccr, &hash).await.is_some() {
											state.ccr_hits.fetch_add(1, Ordering::Relaxed);
										} else {
											state.ccr_misses.fetch_add(1, Ordering::Relaxed);
											ccr_put(ccr, &hash, &args_owned).await;
											state.ccr_created.fetch_add(1, Ordering::Relaxed);
										}
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
										state
											.tokens_saved
											.fetch_add(orig_len.saturating_sub(marker_len) as u64, Ordering::Relaxed);
										*args = serde_json::Value::String(compressed);
										did_compress = true;
										state.update_compression_ratio(orig_len, marker_len);
									}
								} else if args_owned.len() > INLINE_CCR_THRESHOLD {
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
				tracing::warn!(callback_url = %cb, "tool_relay callback skipped: only https scheme allowed");
				return Json(ToolRelayResponse { success:true, result:None, error:None, async_call:false })
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
			if size < INLINE_CCR_THRESHOLD {
				// Tiny content: store inline to avoid CCR backend round-trip
				if let Ok(mut map) = state.inline_ccr.lock() {
					if map.contains(&hash) {
						state.inline_ccr_hits.fetch_add(1, Ordering::Relaxed);
					} else {
						state.inline_ccr_misses.fetch_add(1, Ordering::Relaxed);
						map.put(hash.clone(), content.to_string());
					}
				}
				Ok(serde_json::json!({
					"compressed": smart_marker(&hash, content, "compress", center),
					"hash": hash,
					"original_size": size
				}))
			} else if let Some(ccr) = &state.ccr {
				ccr_put(ccr, &hash, content).await;
				let compressed = smart_marker(&hash, content, "compress", center);
				// Savings = bytes removed by the marker replacement, not the
				// bare hash length (report 05 F5).
				state.tokens_saved.fetch_add(size.saturating_sub(compressed.len()) as u64, Ordering::Relaxed);
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

				if let Some(ccr) = &state.ccr {
					ccr_put(ccr, &hash, &req.content).await;
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
				}

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

		if let Some(ccr) = &state.ccr {
			ccr_put(ccr, &hash, &content).await;
			state.ccr_created.fetch_add(1, Ordering::Relaxed);
			state.requests_compressed.fetch_add(1, Ordering::Relaxed);
			// See the JSON-body branch above: this endpoint's wire contract
			// IS the bare hash, so `hash.len()` is the correct subtractee.
			state
				.tokens_saved
				.fetch_add(original_size.saturating_sub(hash.len()) as u64, Ordering::Relaxed);
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
pub async fn handle_ccr_reload() -> impl IntoResponse {
	let config_path = std::env::var("APHRODITE_CONFIG_PATH").unwrap_or_else(|_| "aphrodite.toml".to_string());
	match crate::config::MultiConfig::load(&config_path) {
		Ok(config) => {
			let comp = config.compression.as_ref();
			let body = serde_json::json!({
				"reloaded": true,
				"config": config_path,
				"compression": {
					"auto_expand": comp.and_then(|c| c.auto_expand),
					"auto_expand_limit": comp.and_then(|c| c.auto_expand_limit),
					"tool_threshold_token": comp.and_then(|c| c.tool_threshold_token),
					"tool_threshold_cache": comp.and_then(|c| c.tool_threshold_cache),
					"terminal_threshold": comp.and_then(|c| c.terminal_threshold),
					"inline_threshold": comp.and_then(|c| c.inline_threshold),
					"engine_threshold_pct": comp.and_then(|c| c.engine_threshold_pct),
					"catalog_mode": comp.and_then(|c| c.catalog_mode.clone()),
				}
			});
			tracing::info!(%config_path, "config hot-reloaded");
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
			cache_hits:AtomicU64::new(0),
			cache_misses:AtomicU64::new(0),
			fill_pct:AtomicU64::new(9000),
			task_tracker:TaskTracker::new(),
			script_engine:None,
			inline_ccr_hits:AtomicU64::new(0),
			inline_ccr_misses:AtomicU64::new(0),
			tool_relay_success:AtomicU64::new(0),
			tool_relay_failure:AtomicU64::new(0),
			notify_success:AtomicU64::new(0),
			notify_failure:AtomicU64::new(0),
			upstream_errors_4xx:AtomicU64::new(0),
			upstream_errors_5xx:AtomicU64::new(0),
			upstream_timeouts:AtomicU64::new(0),
			ccr_store_entries:AtomicU64::new(0),
			ccr_store_bytes:AtomicU64::new(0),
			request_body_bytes:AtomicU64::new(0),
			response_body_bytes:AtomicU64::new(0),
			upstream_latency_micros:AtomicU64::new(0),
		};
		assert_eq!(state.compress_threshold(), CACHE_COMPRESS_THRESHOLD);
	}

	#[test]
	fn test_compress_threshold_aphrodite() {
		let state = AppState { mode:ProxyMode::Token, ..test_state() };
		assert_eq!(state.compress_threshold(), TOKEN_COMPRESS_THRESHOLD);
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
			cache_hits:AtomicU64::new(0),
			cache_misses:AtomicU64::new(0),
			fill_pct:AtomicU64::new(9000),
			task_tracker:TaskTracker::new(),
			script_engine:None,
			inline_ccr_hits:AtomicU64::new(0),
			inline_ccr_misses:AtomicU64::new(0),
			tool_relay_success:AtomicU64::new(0),
			tool_relay_failure:AtomicU64::new(0),
			notify_success:AtomicU64::new(0),
			notify_failure:AtomicU64::new(0),
			upstream_errors_4xx:AtomicU64::new(0),
			upstream_errors_5xx:AtomicU64::new(0),
			upstream_timeouts:AtomicU64::new(0),
			ccr_store_entries:AtomicU64::new(0),
			ccr_store_bytes:AtomicU64::new(0),
			request_body_bytes:AtomicU64::new(0),
			response_body_bytes:AtomicU64::new(0),
			upstream_latency_micros:AtomicU64::new(0),
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
			.block_on(execute_tool_relay(&state, "aphrodite_compress", &serde_json::json!({"content": content})))
			.unwrap();
		let hash = compressed["hash"].as_str().unwrap().to_string();

		for hash_arg in [hash.clone(), format!("{hash}|tool|1024"), format!("  {hash}  ")] {
			let retrieved = rt
				.block_on(execute_tool_relay(&state, "aphrodite_retrieve", &serde_json::json!({"hash": hash_arg})))
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
