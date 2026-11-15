//! The response-cache lookup on the request path.
//!
//! A hit is answered from memory without an upstream round-trip; a miss only
//! bumps the counter and lets the request through. Both are recorded - a hit
//! that skipped `RecordLatency`/`RecordRequest` skewed p50 upward and
//! under-reported traffic in `/history` (F18) - and neither is confused with a
//! bypass, which is counted as nothing because nothing was asked.
//!
//! `Now` is passed in so the whole lookup agrees on one instant; the caller
//! that has to evict is the caller that owns the clock.

use std::{sync::atomic::Ordering, time::Instant};

use axum::{body::Body, http::StatusCode, response::Response};

use crate::Proxy::{
	Cache::{Get::Fn::Fn as ResponseCacheGet, Lookup::Enum::Enum as CacheLookup},
	Log::Dev::Fn::Fn as DevLog,
	Request::{Cacheability::Enum::Enum as RequestCacheability, Context::Struct::Struct as RequestContext},
};

/// Answer the request from the response cache when it may be, and say which of
/// the three cases it was.
pub(crate) fn Fn(Context:&RequestContext<'_>, Now:Instant) -> CacheLookup {
	let Key = match Context.Semantics.Cacheability {
		RequestCacheability::Cacheable { Key } => Key,
		RequestCacheability::Uncacheable => return CacheLookup::Bypassed,
	};

	match ResponseCacheGet(Context.State, Key, Now) {
		Some(CachedBody) => {
			Context.State.CacheHits.fetch_add(1, Ordering::Relaxed);
			// Bytes throughout (report 05 F5): a full upstream round-trip was
			// avoided, so the whole cached response body counts as saved -
			// previously this divided by 4 to estimate a token count while
			// every other `TokensSaved` site counted raw bytes, making the
			// field internally inconsistent by 4x.
			Context.State.TokensSaved.fetch_add(CachedBody.len() as u64, Ordering::Relaxed);
			DevLog(
				Context.State.Dev,
				|| tracing::info!(ID = %Context.ID, CachedLength = CachedBody.len(), "<<< CACHE HIT"),
			);

			// F18: cache hits used to skip both of these entirely, so
			// `/history` and the latency histogram never saw them.
			Context.State.RecordLatency(Context.Started.elapsed());
			Context.State.RecordRequest(
				Context.ID,
				Context.Method.as_str(),
				Context.Path,
				200,
				false,
				Context.Started.elapsed().as_millis(),
			);

			CacheLookup::Hit(
				Response::builder()
					.status(StatusCode::OK)
					.header("Content-Type", "application/json; charset=utf-8")
					.header("X-Aphrodite-Cache", "HIT")
					.header("X-Aphrodite-Fill-Pct", Context.State.FillPctText())
					.body(Body::from(CachedBody))
					.unwrap(),
			)
		},
		None => {
			Context.State.CacheMisses.fetch_add(1, Ordering::Relaxed);

			CacheLookup::Miss { Key }
		},
	}
}
