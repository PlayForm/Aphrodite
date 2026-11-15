//! `AppState::stats_json` - the `/stats` payload.
//!
//! Built field by field so every counter group has a stable JSON key. Live
//! store values (entries, bytes) are queried here rather than tracked as
//! atomics: a counter nothing increments reports a permanent zero, which is
//! the defect this shape exists to avoid.

use std::sync::atomic::Ordering;

use crate::{Configuration::Proxy::Mode::Enum::Enum as ProxyMode, Proxy::State::Struct::Struct as AppState};

impl AppState {
	pub fn StatsJSON(&self) -> serde_json::Value {
		serde_json::json!({
			"mode": match self.Mode {
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
			"tool_relay_enabled": self.ToolRelay,
			"requests": {
				"total": self.RequestsTotal.load(Ordering::Relaxed),
				"compressed": self.RequestsCompressed.load(Ordering::Relaxed),
			},
			"tokens_saved": self.TokensSaved.load(Ordering::Relaxed),
			"ccr": {
				"hits": self.CCRHits.load(Ordering::Relaxed),
				"misses": self.CCRMisses.load(Ordering::Relaxed),
				"created": self.CCRCreated.load(Ordering::Relaxed),
			},
			"tool_relay_calls": self.ToolRelayCalls.load(Ordering::Relaxed),
			"cache": {
				"hits": self.CacheHits.load(Ordering::Relaxed),
				"misses": self.CacheMisses.load(Ordering::Relaxed),
			},
			"latency_buckets_us": [
				self.LatencyBuckets[0].load(Ordering::Relaxed),
				self.LatencyBuckets[1].load(Ordering::Relaxed),
				self.LatencyBuckets[2].load(Ordering::Relaxed),
				self.LatencyBuckets[3].load(Ordering::Relaxed),
				self.LatencyBuckets[4].load(Ordering::Relaxed),
			],
			"total_latency_micros": self.TotalLatencyMicros.load(Ordering::Relaxed),
			"compressions_by_type": self.CompressionsByType.lock().map(|m| m.clone()).unwrap_or_default(),
			"compression_ratio_ema": self.CompressionRatioEma.load(Ordering::Relaxed) as f64 / 100.0,
			"last_errors": self.LastErrors.lock().map(|v| v.iter().rev().take(5).cloned().collect::<Vec<_>>()).unwrap_or_default(),
			"request_history": self.RequestHistory.lock().map(|v| v.clone()).unwrap_or_default(),
			"inline_ccr": {
				"hits": self.InlineCCRHits.load(Ordering::Relaxed),
				"misses": self.InlineCCRMisses.load(Ordering::Relaxed),
			},
			"tool_relay": {
				"total": self.ToolRelayCalls.load(Ordering::Relaxed),
				"success": self.ToolRelaySuccess.load(Ordering::Relaxed),
				"failure": self.ToolRelayFailure.load(Ordering::Relaxed),
			},
			"notify": {
				"success": self.NotifySuccess.load(Ordering::Relaxed),
				"failure": self.NotifyFailure.load(Ordering::Relaxed),
			},
			"upstream_errors": {
				"4xx": self.UpstreamErrors4xx.load(Ordering::Relaxed),
				"5xx": self.UpstreamErrors5xx.load(Ordering::Relaxed),
				"timeouts": self.UpstreamTimeouts.load(Ordering::Relaxed),
				// F17: non-timeout transport failures (connect refused, DNS,
				// TLS) - previously folded into "timeouts" above.
				"connect_errors": self.UpstreamConnectErrors.load(Ordering::Relaxed),
				// 02-F9: mid-stream chunk errors on the SSE relay path.
				"sse_stream_errors": self.SSEStreamErrors.load(Ordering::Relaxed),
			},
			"ccr_store": {
				// Live store values, not counters: these two fields used to
				// be atomics that nothing ever incremented, so `/stats`
				// reported a permanent `entries: 0` no matter how full the
				// store was - the same class of defect as `FillPct` being
				// read as store occupancy. `len()` is O(1) (maintained by
				// the backend) and the byte total comes from the backend's
				// own stats query, which `/stats` can afford.
				"entries": self.ccr.as_ref().map(|c| c.len()).unwrap_or(0),
				"bytes_approx": self
					.ccr
					.as_ref()
					.and_then(|c| c.stats_db())
					.and_then(|v| v.get("total_bytes_original").and_then(|n| n.as_u64()))
					.unwrap_or(0),
			},
			"body_bytes": {
				"request": self.RequestBodyBytes.load(Ordering::Relaxed),
				"response": self.ResponseBodyBytes.load(Ordering::Relaxed),
			},
			"upstream_latency_micros": self.UpstreamLatencyMicros.load(Ordering::Relaxed),
		})
	}
}
