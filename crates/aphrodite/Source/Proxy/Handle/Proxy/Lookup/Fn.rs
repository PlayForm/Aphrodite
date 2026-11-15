//! //! The response-cache lookup on the request path.
//!
//! A hit is answered from memory without an upstream round-trip; a miss only
//! bumps the counter and lets the request through. Both branches are recorded -
//! a hit that skipped `RecordLatency`/`RecordRequest` skewed p50 upward and
//! under-reported traffic in `/history` (F18).

use std::sync::{Arc, atomic::Ordering};

use axum::{
	body::Body,
	http::{Method, StatusCode},
	response::Response,
};

use crate::Proxy::{Cache::Get::Fn::Fn as ResponseCacheGet, State::Struct::Struct as AppState};

pub(crate) fn Fn(
	State:&Arc<AppState>,
	CacheKey:Option<u64>,
	RequestIDShort:&str,
	Method:&Method,
	path:&str,
	T0:std::time::Instant,
) -> Option<Response> {
	// Check LLM API response cache before upstream call
	if let Some(Ck) = CacheKey {
		let CachedBody = ResponseCacheGet(State, Ck);
		if let Some(CachedBody) = CachedBody {
			State.CacheHits.fetch_add(1, Ordering::Relaxed);
			// Bytes throughout (report 05 F5): a full upstream round-trip
			// was avoided, so the whole cached response body counts as
			// saved - previously this divided by 4 to estimate a token
			// count while every other `TokensSaved` site counted raw
			// bytes, making the field internally inconsistent by 4x.
			State.TokensSaved.fetch_add(CachedBody.len() as u64, Ordering::Relaxed);
			if State.Dev {
				tracing::info!(
					ID = %RequestIDShort,
					CachedLength = CachedBody.len(),
					"<<< CACHE HIT"
				);
			}
			// F18: cache hits used to skip both of these entirely, so
			// `/history` and the latency histogram never saw them - the p50
			// skewed upward (only ever measuring cache MISSES) and the
			// request-history ring buffer under-reported real traffic.
			State.RecordLatency(T0.elapsed());
			State.RecordRequest(RequestIDShort, Method.as_str(), path, 200, false, T0.elapsed().as_millis());
			return Some(
				Response::builder()
					.status(StatusCode::OK)
					.header("Content-Type", "application/json; charset=utf-8")
					.header("X-Aphrodite-Cache", "HIT")
					.header("X-Aphrodite-Fill-Pct", {
						let v = State.FillPct.load(Ordering::Relaxed) as f64 / 100.0;
						if v.is_finite() { format!("{:.1}", v) } else { "0.0".to_string() }
					})
					.body(Body::from(CachedBody))
					.unwrap(),
			);
		} else {
			State.CacheMisses.fetch_add(1, Ordering::Relaxed);
			if State.Dev {
				tracing::info!(
					ID = %RequestIDShort,
					"<<< CACHE MISS"
				);
			}
		}
	}
	None
}
