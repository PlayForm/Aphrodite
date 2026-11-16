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
	Cache::{Get::Fn::Fn as Get, Lookup::Enum::Enum as CacheLookup},
	Log::Dev::Fn::Fn as DevLog,
	Measurement::Encoded::Struct::Struct as EncodedAccounting,
	Request::{Cacheability::Enum::Enum as RequestCacheability, Context::Struct::Struct as RequestContext},
};

/// Answer the request from the response cache when it may be, and say which of
/// the three cases it was.
pub(crate) fn Fn(Context:&RequestContext<'_>, Now:Instant) -> CacheLookup {
	let Identity = match &Context.Semantics.Cacheability {
		RequestCacheability::Cacheable { Identity } => Identity,
		RequestCacheability::Uncacheable => return CacheLookup::Bypassed,
	};

	match Get(Context.State, Identity, Now) {
		Some(CachedBody) => {
			Context.State.CacheHits.fetch_add(1, Ordering::Relaxed);
			// Encoded-body accounting, not content accounting: a hit avoided a
			// whole upstream round-trip, so the serialised body never crossed the
			// wire. This used to be added to `TokensSaved`, which counts the
			// content bytes a marker replaced - two measurements with two
			// different denominators in one counter.
			let Avoided = EncodedAccounting { BodyBytes:CachedBody.len() }.Avoided();

			Context.State.BytesSaved.fetch_add(Avoided, Ordering::Relaxed);
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
					.header("X-Aphrodite-Fill-Pct", Context.State.FillText())
					.body(Body::from(CachedBody))
					.unwrap(),
			)
		},
		None => {
			Context.State.CacheMisses.fetch_add(1, Ordering::Relaxed);

			// The whole identity, not its digest: the writer stores exactly what a
			// hit is decided on. Cloning it is a `Bytes` refcount bump.
			CacheLookup::Miss { Identity:Identity.clone() }
		},
	}
}
