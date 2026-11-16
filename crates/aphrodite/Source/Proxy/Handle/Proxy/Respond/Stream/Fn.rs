//! Relay an SSE body without buffering it.
//!
//! T10 (F6): a `text/event-stream` response must be forwarded chunk by chunk -
//! buffering it into one JSON blob produces something an SSE client cannot
//! parse. Compression and caching are skipped entirely on this path: there is no
//! single body to store, and a cached entry replayed as `application/json` is
//! nothing like the stream the caller asked for.
//!
//! 02-F9: bytes are counted and mid-stream errors observed as chunks flow
//! through. Once `Body::from_stream` owns the raw `bytes_stream()`, a later
//! chunk `Err` (F2's failure mode) and every SSE byte were otherwise invisible
//! to `/stats`.
//!
//! Accounting is deliberately unchanged: the latency recorded here is
//! time-to-headers, not the duration of the stream, because the stream outlives
//! this function. Reinterpreting it would be a metrics-contract change - but the
//! code says which half it is describing, so a reader cannot compare it with a
//! buffered request's whole-body latency by mistake. The byte counter below is the
//! other half: it counts BODY bytes as they pass, which is why the two numbers
//! describe different parts of one response rather than one duration.

use std::sync::atomic::Ordering;

use axum::{body::Body, response::Response};
use futures::StreamExt;

use crate::Proxy::{
	Log::Dev::Fn::Fn as DevLog,
	Request::Context::Struct::Struct as RequestContext,
	Upstream::{Head::Struct::Struct as UpstreamHead, Headers::Fn::Fn as CopyHeaders},
};

/// Relay the upstream stream to the client.
pub(crate) fn Fn(Context:&RequestContext<'_>, Head:UpstreamHead, Response:reqwest::Response) -> Response {
	let State = Context.State.clone();
	let Stream = Response.bytes_stream().inspect(move |Chunk| {
		match Chunk {
			Ok(bytes) => {
				State.ResponseBytes.fetch_add(bytes.len() as u64, Ordering::Relaxed);
			},
			Err(_) => {
				State.StreamErrors.fetch_add(1, Ordering::Relaxed);
			},
		}
	});

	DevLog(
		Context.State.Dev,
		|| tracing::info!(ID = %Context.ID, Status = %Head.Status, "<<< STREAM (SSE)"),
	);

	// Time to HEADERS, not the duration of the stream: the stream outlives this
	// function, so the name is what keeps this number from being read as a
	// whole-response latency beside the buffered path's.
	let Elapsed = Context.Started.elapsed();

	Context.State.RecordLatency(Elapsed);
	Context.State.RecordRequest(
		Context.ID,
		Context.Method.as_str(),
		Context.Path,
		Head.Status.as_u16(),
		false,
		Elapsed.as_millis(),
	);

	let Builder = CopyHeaders(Response::builder().status(Head.Status), &Head.Headers);
	let Builder = match Head.ContentType.clone() {
		Some(Ct) => Builder.header("Content-Type", Ct),
		None => Builder,
	};

	Builder
		.header("X-Aphrodite-Streamed", "true")
		.body(Body::from_stream(Stream))
		.unwrap()
}
