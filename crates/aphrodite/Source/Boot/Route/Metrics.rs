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
	match Stats["cache"].as_object() {
		Some(Cache) => {
			Out.push_str(&format!("aphrodite_cache_hits_total {}\n", Cache["hits"]));
			Out.push_str(&format!("aphrodite_cache_misses_total {}\n", Cache["misses"]));
		},
		None => {},
	}
	// Latency buckets
	match Stats["latency_buckets_us"].as_array() {
		Some(Buckets) => {
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
			match Stats["total_latency_micros"].as_u64() {
				Some(TotalUs) => {
					Out.push_str(&format!("aphrodite_latency_seconds_sum {:.6}\n", TotalUs as f64 / 1_000_000.0));
				},
				None => {},
			}
		},
		None => {},
	}
	match Stats["compression_ratio_ema"].as_f64() {
		Some(Ratio) => {
			Out.push_str(&format!("aphrodite_compression_ratio_ema {:.2}\n", Ratio));
		},
		None => {},
	}
	// Inline CCR
	match Stats["inline_ccr"].as_object() {
		Some(Icc) => {
			match Icc["hits"].as_u64() {
				Some(h) => {
					Out.push_str(&format!("aphrodite_inline_ccr_hits_total {h}\n"));
				},
				None => {},
			}
			match Icc["misses"].as_u64() {
				Some(m) => {
					Out.push_str(&format!("aphrodite_inline_ccr_misses_total {m}\n"));
				},
				None => {},
			}
		},
		None => {},
	}
	// Tool relay success/failure
	match Stats["tool_relay"].as_object() {
		Some(Tr) => {
			match Tr["success"].as_u64() {
				Some(s) => {
					Out.push_str(&format!("aphrodite_tool_relay_success_total {s}\n"));
				},
				None => {},
			}
			match Tr["failure"].as_u64() {
				Some(f) => {
					Out.push_str(&format!("aphrodite_tool_relay_failure_total {f}\n"));
				},
				None => {},
			}
		},
		None => {},
	}
	// Notify success/failure
	match Stats["notify"].as_object() {
		Some(n) => {
			match n["success"].as_u64() {
				Some(s) => {
					Out.push_str(&format!("aphrodite_notify_success_total {s}\n"));
				},
				None => {},
			}
			match n["failure"].as_u64() {
				Some(f) => {
					Out.push_str(&format!("aphrodite_notify_failure_total {f}\n"));
				},
				None => {},
			}
		},
		None => {},
	}
	// Upstream errors
	match Stats["upstream_errors"].as_object() {
		Some(Ue) => {
			match Ue["4xx"].as_u64() {
				Some(c) => {
					Out.push_str(&format!("aphrodite_upstream_errors_total{{code=\"4xx\"}} {c}\n"));
				},
				None => {},
			}
			match Ue["5xx"].as_u64() {
				Some(c) => {
					Out.push_str(&format!("aphrodite_upstream_errors_total{{code=\"5xx\"}} {c}\n"));
				},
				None => {},
			}
			match Ue["timeouts"].as_u64() {
				Some(t) => {
					Out.push_str(&format!("aphrodite_upstream_timeouts_total {t}\n"));
				},
				None => {},
			}
			match Ue["connect_errors"].as_u64() {
				Some(c) => {
					Out.push_str(&format!("aphrodite_upstream_connect_errors_total {c}\n"));
				},
				None => {},
			}
			// 02-F9: mid-stream SSE chunk errors - added to /stats when
			// sse_stream_errors was introduced, but missed here.
			match Ue["sse_stream_errors"].as_u64() {
				Some(c) => {
					Out.push_str(&format!("aphrodite_sse_stream_errors_total {c}\n"));
				},
				None => {},
			}
		},
		None => {},
	}
	// CCR store info
	match Stats["ccr_store"].as_object() {
		Some(Cs) => {
			match Cs["entries"].as_u64() {
				Some(e) => {
					Out.push_str(&format!("aphrodite_ccr_store_entries {e}\n"));
				},
				None => {},
			}
			match Cs["bytes_approx"].as_u64() {
				Some(b) => {
					Out.push_str(&format!("aphrodite_ccr_store_bytes {b}\n"));
				},
				None => {},
			}
		},
		None => {},
	}
	// Body bytes
	match Stats["body_bytes"].as_object() {
		Some(Bb) => {
			match Bb["request"].as_u64() {
				Some(r) => {
					Out.push_str(&format!("aphrodite_request_body_bytes_total {r}\n"));
				},
				None => {},
			}
			match Bb["response"].as_u64() {
				Some(r) => {
					Out.push_str(&format!("aphrodite_response_body_bytes_total {r}\n"));
				},
				None => {},
			}
		},
		None => {},
	}
	// Upstream latency
	match Stats["upstream_latency_micros"].as_u64() {
		Some(Ul) => {
			Out.push_str(&format!(
				"aphrodite_upstream_latency_seconds_total {:.6}\n",
				Ul as f64 / 1_000_000.0
			));
		},
		None => {},
	}
	Out
}
