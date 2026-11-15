//! `/metrics` - the Prometheus text exposition.

use std::sync::Arc;

use aphrodite::Proxy::State::Struct::Struct as AppState;
use axum::{Router, http::StatusCode, routing::get};

/// `/metrics` - the statistics in Prometheus text format.
///
/// Restricted by the loopback + `Host` layer but exempt from the bearer gate:
/// a Prometheus scraper typically cannot be configured to send a token. It
/// exposes the same counters as `/stats`, which is NOT exempt - the difference is
/// that `/stats` also carries request history, and a scraper has no business
/// reading that.
pub fn Route(State:&Arc<AppState>) -> Router<Arc<AppState>> {
	Router::new().route(
		"/metrics",
		get({
			let s = State.clone();
			move || {
				async move {
					let Stats = s.StatsJSON();
					(
						StatusCode::OK,
						[(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
						body(&Stats),
					)
				}
			}
		}),
	)
}

/// Render the stats document as Prometheus text exposition.
///
/// Kept as a pure function of the stats JSON so the exposition format can be
/// checked without an HTTP request. Two label choices are load-bearing and were
/// both wrong before: the last latency bucket is labelled `+Inf` because it has no
/// upper bound (`RecordLatency`'s final arm catches every sample >= 1s, so
/// calling it `10.0` made `histogram_quantile()` compute nonsense), and the
/// explicit `+Inf` bucket is what makes `_count` agree with the bucket sum.
///
/// Every section is an absent-or-present optional member of the stats document,
/// so each one reads as `if let Some(..)` and a missing member emits no line at
/// all. Nothing here branches on a value: the shape of the document is the only
/// thing that decides which metrics exist.
fn body(Stats:&serde_json::Value) -> String {
	let mut Out = String::new();
	let ModeStr = Stats["mode"].as_str().unwrap_or("unknown");
	Out.push_str(&format!(
		"aphrodite_requests_total{{mode=\"{}\"}} {}\n",
		ModeStr, Stats["requests"]["total"]
	));
	Out.push_str(&format!(
		"aphrodite_requests_compressed_total{{mode=\"{}\"}} {}\n",
		ModeStr, Stats["requests"]["compressed"]
	));
	Out.push_str(&format!("aphrodite_tokens_saved_total {}\n", Stats["tokens_saved"]));
	Out.push_str(&format!("aphrodite_ccr_hits_total {}\n", Stats["ccr"]["hits"]));
	Out.push_str(&format!("aphrodite_ccr_misses_total {}\n", Stats["ccr"]["misses"]));
	Out.push_str(&format!("aphrodite_ccr_created_total {}\n", Stats["ccr"]["created"]));
	Out.push_str(&format!("aphrodite_tool_relay_calls_total {}\n", Stats["tool_relay_calls"]));
	// LLM response cache
	if let Some(Cache) = Stats["cache"].as_object() {
		Out.push_str(&format!("aphrodite_cache_hits_total {}\n", Cache["hits"]));
		Out.push_str(&format!("aphrodite_cache_misses_total {}\n", Cache["misses"]));
	}
	// Latency buckets
	if let Some(Buckets) = Stats["latency_buckets_us"].as_array() {
		let mut TotalCount:u64 = 0;
		// F11: bucket 4 (`RecordLatency`'s last arm) has no
		// upper bound - it catches every sample >= 1s, including
		// 30s+ outliers - so labeling it "10.0" was actively
		// wrong (Prometheus consumers would assume everything in
		// it is <= 10s and mis-compute quantiles), and the
		// missing explicit "+Inf" bucket (required to equal
		// `_count`) broke `histogram_quantile()`. Relabeling
		// bucket 4 as "+Inf" is honest about its real bound and
		// satisfies the convention in one step - no new bucket
		// needed since it was already unbounded in practice.
		let LeLabels = ["0.001", "0.01", "0.1", "1.0", "+Inf"];
		Buckets.iter().enumerate().for_each(|(i, v)| {
			let Le = LeLabels.get(i).copied().unwrap_or("+Inf");
			let Count = v.as_u64().unwrap_or(0);
			TotalCount += Count;
			Out.push_str(&format!("aphrodite_latency_seconds_bucket{{le=\"{}\"}} {}\n", Le, TotalCount));
		});
		Out.push_str(&format!("aphrodite_latency_seconds_count {}\n", TotalCount));
		if let Some(TotalUs) = Stats["total_latency_micros"].as_u64() {
			Out.push_str(&format!("aphrodite_latency_seconds_sum {:.6}\n", TotalUs as f64 / 1_000_000.0));
		}
	}
	if let Some(Ratio) = Stats["compression_ratio_ema"].as_f64() {
		Out.push_str(&format!("aphrodite_compression_ratio_ema {:.2}\n", Ratio));
	}
	// Inline CCR
	if let Some(Icc) = Stats["inline_ccr"].as_object() {
		if let Some(h) = Icc["hits"].as_u64() {
			Out.push_str(&format!("aphrodite_inline_ccr_hits_total {h}\n"));
		}
		if let Some(m) = Icc["misses"].as_u64() {
			Out.push_str(&format!("aphrodite_inline_ccr_misses_total {m}\n"));
		}
	}
	// Tool relay success/failure
	if let Some(Tr) = Stats["tool_relay"].as_object() {
		if let Some(s) = Tr["success"].as_u64() {
			Out.push_str(&format!("aphrodite_tool_relay_success_total {s}\n"));
		}
		if let Some(f) = Tr["failure"].as_u64() {
			Out.push_str(&format!("aphrodite_tool_relay_failure_total {f}\n"));
		}
	}
	// Notify success/failure
	if let Some(n) = Stats["notify"].as_object() {
		if let Some(s) = n["success"].as_u64() {
			Out.push_str(&format!("aphrodite_notify_success_total {s}\n"));
		}
		if let Some(f) = n["failure"].as_u64() {
			Out.push_str(&format!("aphrodite_notify_failure_total {f}\n"));
		}
	}
	// Upstream errors
	if let Some(Ue) = Stats["upstream_errors"].as_object() {
		if let Some(c) = Ue["4xx"].as_u64() {
			Out.push_str(&format!("aphrodite_upstream_errors_total{{code=\"4xx\"}} {c}\n"));
		}
		if let Some(c) = Ue["5xx"].as_u64() {
			Out.push_str(&format!("aphrodite_upstream_errors_total{{code=\"5xx\"}} {c}\n"));
		}
		if let Some(t) = Ue["timeouts"].as_u64() {
			Out.push_str(&format!("aphrodite_upstream_timeouts_total {t}\n"));
		}
		if let Some(c) = Ue["connect_errors"].as_u64() {
			Out.push_str(&format!("aphrodite_upstream_connect_errors_total {c}\n"));
		}
		// 02-F9: mid-stream SSE chunk errors - added to /stats when
		// sse_stream_errors was introduced, but missed here.
		if let Some(c) = Ue["sse_stream_errors"].as_u64() {
			Out.push_str(&format!("aphrodite_sse_stream_errors_total {c}\n"));
		}
	}
	// CCR store info
	if let Some(Cs) = Stats["ccr_store"].as_object() {
		if let Some(e) = Cs["entries"].as_u64() {
			Out.push_str(&format!("aphrodite_ccr_store_entries {e}\n"));
		}
		if let Some(b) = Cs["bytes_approx"].as_u64() {
			Out.push_str(&format!("aphrodite_ccr_store_bytes {b}\n"));
		}
	}
	// Body bytes
	if let Some(Bb) = Stats["body_bytes"].as_object() {
		if let Some(r) = Bb["request"].as_u64() {
			Out.push_str(&format!("aphrodite_request_body_bytes_total {r}\n"));
		}
		if let Some(r) = Bb["response"].as_u64() {
			Out.push_str(&format!("aphrodite_response_body_bytes_total {r}\n"));
		}
	}
	// Upstream latency
	if let Some(Ul) = Stats["upstream_latency_micros"].as_u64() {
		Out.push_str(&format!(
			"aphrodite_upstream_latency_seconds_total {:.6}\n",
			Ul as f64 / 1_000_000.0
		));
	}
	Out
}
