//! Accumulate the upstream body, rewrite it, and render the client response.
//!
//! The order is the contract: read under the cap, record upstream latency and
//! bytes, then rewrite, then select the bytes, then account, then cache. In
//! particular the savings counter is incremented from the *selected* body, so
//! it describes what was emitted rather than what a rewrite intended - a
//! transform that failed to serialise emits the original and saves nothing.
//!
//! Caching rules are unchanged and both are load-bearing:
//! - success only (F2): caching an upstream 4xx/5xx would replay it as a 200 to
//!   every later identical request, and a client would parse the error body as
//!   a real chat completion;
//! - byte-capped (report 06 F7): `ResponseCache` is count-bounded (128 entries)
//!   but not byte-bounded, so 128 large completions could hold well over 100MB.
//!
//! `Payload` is a `Bytes` handle, so the cache entry, the response body and the
//! length check share one allocation instead of copying the completion.

use std::sync::atomic::Ordering;

use axum::{body::Body, response::Response};

use crate::Proxy::{
	Compress::Chat::Fn::Fn as CompressChatCompletion,
	Handle::Proxy::Respond::{
		BadGateway::Fn::Fn as BadGateway,
		FinalBody::{Enum::Enum as FinalBody, Select::Fn::Fn as SelectFinalBody},
	},
	Log::Dev::Fn::Fn as DevLog,
	Request::{
		Cacheability::Enum::Enum as RequestCacheability,
		Context::Struct::Struct as RequestContext,
		Route::Enum::Enum as RequestRoute,
	},
	Threshold::Const::{ResponseCacheMaximumBodyBytes, ResponseMaximumBodyBytes},
	Upstream::{
		Body::Fn::Fn as AccumulateBody,
		Head::Struct::Struct as UpstreamHead,
		Headers::Fn::Fn as CopyUpstreamHeaders,
	},
};

/// Render a buffered upstream response.
pub(crate) async fn Fn(Context:&RequestContext<'_>, Head:UpstreamHead, Response:reqwest::Response) -> Response {
	let Original = match AccumulateBody(Response, ResponseMaximumBodyBytes).await {
		Ok(Body) => Body,
		Err(e) => {
			// 02-F8: the detailed error (which can embed the upstream
			// URL/host) goes server-side only, via `RecordError` - the client
			// gets a generic message. `/stats`'s `LastErrors` still has the
			// real string for an operator to diagnose from.
			Context.State.RecordError(format!("body read: {e:#}"));

			return BadGateway();
		},
	};

	// Track upstream latency (before compression) and the response body bytes.
	Context
		.State
		.UpstreamLatencyMicros
		.fetch_add(Context.Started.elapsed().as_micros() as u64, Ordering::Relaxed);
	Context
		.State
		.ResponseBodyBytes
		.fetch_add(Original.len() as u64, Ordering::Relaxed);

	// Only a Chat Completions response has a shape the compression pass
	// understands. `Compress::Chat` itself declines when no store is
	// configured, so there is nothing to gate here beyond the route.
	let Outcome = match Context.Semantics.Route {
		RequestRoute::ChatCompletions => {
			// Extract headroom budget from inbound headers for compression
			// aggressiveness. `HeaderMap::get` is already case-insensitive
			// (F20), so a second case-sensitive lookup was permanently dead.
			let HeadroomBudget = Context.Headers.get("x-headroom-budget").and_then(|Value| Value.to_str().ok());
			DevLog(Context.State.Dev && HeadroomBudget.is_some(), || {
				tracing::info!(
					ID = %Context.ID,
					Budget = %HeadroomBudget.unwrap_or(""),
					"headroom budget applied to compression threshold"
				)
			});

			CompressChatCompletion(Context, &Original, HeadroomBudget).await
		},
		RequestRoute::Other => None,
	};

	let FinalBody = SelectFinalBody(&Original, Outcome);
	let Compressed = matches!(FinalBody, FinalBody::Transformed { .. });

	let Builder = match &FinalBody {
		FinalBody::Transformed { Accounting, .. } => {
			Context.State.RequestsCompressed.fetch_add(1, Ordering::Relaxed);
			// Wire-level accounting, after final-body selection: savings must
			// describe what was actually emitted.
			Context.State.TokensSaved.fetch_add(Accounting.Saved(), Ordering::Relaxed);

			CopyUpstreamHeaders(Response::builder().status(Head.Status), &Head.Headers)
				.header("Content-Type", "application/json; charset=utf-8")
				.header("X-Aphrodite-Compressed", "true")
				.header("X-Aphrodite-Cache", "MISS")
				.header("X-Aphrodite-Fill-Pct", Context.State.FillPctText())
		},
		FinalBody::Original(_) => {
			let Builder = CopyUpstreamHeaders(Response::builder().status(Head.Status), &Head.Headers)
				.header("X-Aphrodite-Cache", "MISS")
				.header("X-Aphrodite-Fill-Pct", Context.State.FillPctText());

			match Head.ContentType.clone() {
				Some(Ct) => Builder.header("Content-Type", Ct),
				None => Builder,
			}
		},
	};

	// The emitted bytes, once: the dev log, the cache entry and the response body
	// all read this same `Bytes` handle.
	let Payload = FinalBody.Payload();

	DevLog(Context.State.Dev, || {
		let Text = std::str::from_utf8(&Payload).unwrap_or("?");
		let BodyPreview = if Payload.len() > 500 {
			let Preview:String = Text
				.char_indices()
				.take_while(|(Index, _)| *Index < 200)
				.map(|(_, Char)| Char)
				.collect();

			format!("{Preview}... ({} total)", Payload.len())
		} else {
			Text.to_string()
		};

		tracing::info!(
			ID = %Context.ID,
			Status = %Head.Status,
			ResponseLength = Payload.len(),
			ElapsedMs = Context.Started.elapsed().as_millis(),
			body = %BodyPreview,
			"<<< RES"
		);
	});

	Context.State.RecordLatency(Context.Started.elapsed());
	Context.State.RecordRequest(
		Context.ID,
		Context.Method.as_str(),
		Context.Path,
		Head.Status.as_u16(),
		Compressed,
		Context.Started.elapsed().as_millis(),
	);

	let Now = std::time::Instant::now();
	// The identity is what gets stored, so a later lookup compares canonical
	// request bytes instead of trusting a digest match; cloning it is a `Bytes`
	// refcount bump. The response-side gates stay here rather than moving into
	// the request interpretation: the request decided it MAY be cached, the
	// response decides whether this particular answer is worth replaying.
	let Identity = match &Context.Semantics.Cacheability {
		RequestCacheability::Cacheable { Identity } => Some(Identity.clone()),
		RequestCacheability::Uncacheable => None,
	};
	Identity
		.filter(|_| Head.Status.is_success() && Payload.len() <= ResponseCacheMaximumBodyBytes)
		.into_iter()
		.for_each(|Identity| Context.State.CachePut(Identity, Payload.clone(), Now));

	Builder.body(Body::from(Payload)).unwrap()
}
