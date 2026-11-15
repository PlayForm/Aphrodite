//! //! Turn the upstream result into the client response.
//!
//! Three branches that must stay distinct: an SSE body is relayed chunk by
//! chunk and never buffered or cached; a buffered body is compressed when it is
//! a Chat Completions response and the CCR store is enabled; a transport error
//! becomes a 502 with a generic message. The upstream error string is recorded
//! server-side only - it can embed the upstream URL (02-F8).

use std::sync::{Arc, atomic::Ordering};

use axum::{
	body::Body,
	http::{Method, StatusCode},
	response::{IntoResponse, Json, Response},
};
use futures::StreamExt;

use crate::Proxy::{
	Compress::Chat::Fn::Fn as CompressChatCompletion,
	State::Struct::Struct as AppState,
	Stream::Fn::Fn as IsSSE,
	Threshold::Const::{ResponseCacheMaximumBodyBytes, ResponseMaximumBodyBytes},
	Upstream::{Body::Fn::Fn as AccumulateBody, Headers::Fn::Fn as CopyUpstreamHeaders},
};

// The join point of the request context and the upstream outcome. Bundling
// these into a struct would be a larger restructure of the body than this
// extraction allows; each argument is read at most twice.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn Fn(
	State:&Arc<AppState>,
	Method:&Method,
	path:&str,
	RequestIDShort:&str,
	T0:std::time::Instant,
	Headers:&axum::http::HeaderMap,
	CacheKey:Option<u64>,
	IsChatCompletion:bool,
	UpstreamResult:Result<reqwest::Response, String>,
	FinalErrorWasTimeout:bool,
) -> Response {
	match UpstreamResult {
		Ok(response) => {
			let Status = response.status();
			// Track upstream errors by status code
			let StatusCode = Status.as_u16();
			if StatusCode >= 500 {
				State.UpstreamErrors5xx.fetch_add(1, Ordering::Relaxed);
			} else if StatusCode >= 400 {
				State.UpstreamErrors4xx.fetch_add(1, Ordering::Relaxed);
			}
			// F5: capture the full upstream response header map before
			// consuming the body, not just `content-type` - the proxy used
			// to rebuild the response with only `Content-Type`, silently
			// dropping `Retry-After`, `x-ratelimit-*`, `request-id`, etc.
			// that clients rely on for backoff and support.
			let UpstreamHeaders = response.headers().clone();
			let ContentType = UpstreamHeaders.get("content-type").cloned();

			// T10 (F6): SSE streaming - text/event-stream responses must be
			// forwarded chunk-by-chunk, not buffered into a single JSON blob
			// (which an SSE client can't parse). Skip compression + cache
			// entirely for this path.
			if IsSSE(ContentType.as_ref()) {
				// 02-F9: count bytes and observe mid-stream errors as chunks
				// flow through - once `Body::from_stream` owns the raw
				// `bytes_stream()`, a later chunk `Err` (F2's failure mode)
				// and every SSE byte were otherwise invisible to `/stats`.
				let StateForStream = State.clone();
				let Stream = response.bytes_stream().inspect(move |Chunk| {
					match Chunk {
						Ok(bytes) => {
							StateForStream
								.ResponseBodyBytes
								.fetch_add(bytes.len() as u64, Ordering::Relaxed);
						},
						Err(_) => {
							StateForStream.SSEStreamErrors.fetch_add(1, Ordering::Relaxed);
						},
					}
				});
				if State.Dev {
					tracing::info!(ID = %RequestIDShort, Status = %Status, "<<< STREAM (SSE)");
				}
				State.RecordLatency(T0.elapsed());
				State.RecordRequest(
					RequestIDShort,
					Method.as_str(),
					path,
					Status.as_u16(),
					false,
					T0.elapsed().as_millis(),
				);
				let mut Builder = Response::builder().status(Status);
				Builder = CopyUpstreamHeaders(Builder, &UpstreamHeaders);
				if let Some(Ct) = ContentType {
					Builder = Builder.header("Content-Type", Ct);
				}
				Builder = Builder.header("X-Aphrodite-Streamed", "true");
				return Builder.body(Body::from_stream(Stream)).unwrap();
			}

			// Buffer the full response body (non-streaming path). Cap at
			// RESPONSE_MAX_BODY_BYTES to prevent a single huge upstream
			// response from exhausting process memory.
			let ResponseBody = match AccumulateBody(response, ResponseMaximumBodyBytes).await {
				Ok(b) => b,
				Err(e) => {
					// 02-F8: the detailed error (which can embed the
					// upstream URL/host) goes server-side only, via
					// `RecordError` - the client gets a generic message.
					// `/stats`'s `LastErrors` still has the real string for
					// an operator to diagnose from.
					State.RecordError(format!("body read: {}", e));
					return (
						StatusCode::BAD_GATEWAY,
						Json(serde_json::json!({"error": "upstream request failed"})),
					)
						.into_response();
				},
			};

			// Track upstream latency (before compression)
			let UpstreamElapsed = T0.elapsed().as_micros() as u64;
			State.UpstreamLatencyMicros.fetch_add(UpstreamElapsed, Ordering::Relaxed);
			// Track response body bytes
			State.ResponseBodyBytes.fetch_add(ResponseBody.len() as u64, Ordering::Relaxed);

			// Only compress Chat Completions responses
			let Elapsed = T0.elapsed();
			if IsChatCompletion && State.ccr.is_some() {
				// Extract headroom budget from inbound headers for compression
				// aggressiveness. `HeaderMap::get` is already case-insensitive
				// (F20), so the second lookup below used to be permanently
				// dead - removed rather than kept as misleading "just in case"
				// code implying case-sensitive matching.
				let HeadroomBudget = Headers.get("x-headroom-budget").and_then(|v| v.to_str().ok());
				if State.Dev && HeadroomBudget.is_some() {
					tracing::info!(
						ID = %RequestIDShort,
						Budget = %HeadroomBudget.unwrap_or(""),
						"headroom budget applied to compression threshold"
					);
				}
				if let Some(Compressed) = CompressChatCompletion(State, &ResponseBody, HeadroomBudget).await {
					State.RequestsCompressed.fetch_add(1, Ordering::Relaxed);
					State.RecordLatency(Elapsed);
					State.RecordRequest(
						RequestIDShort,
						Method.as_str(),
						path,
						Status.as_u16(),
						true,
						Elapsed.as_millis(),
					);
					if State.Dev {
						let Elapsed = T0.elapsed();
						let CompLength = serde_json::to_vec(&Compressed).map(|v| v.len()).unwrap_or(0);
						tracing::info!(
							ID = %RequestIDShort,
							Status = %Status,
							OriginalLength = ResponseBody.len(),
							CompressedLength = CompLength,
							Ratio = format!("{:.1}x", ResponseBody.len() as f64 / CompLength.max(1) as f64),
							ElapsedMs = Elapsed.as_millis(),
							"<<< COMPRESSED"
						);
					}
					let body = serde_json::to_vec(&Compressed).unwrap_or_else(|_| ResponseBody.to_vec());
					// Store in LLM response cache - only successful responses (F2):
					// caching an upstream 4xx/5xx here would replay it as a 200 to
					// every later identical request, and a client would parse the
					// error body as a real chat completion. TTL-stamped (F5,
					// report 06) so it's checked against `ResponseCacheTtl` on
					// the hit path instead of replaying forever.
					if let Some(Ck) = CacheKey
						&& Status.is_success()
						&& body.len() <= ResponseCacheMaximumBodyBytes
						&& let Ok(mut Cache) = State.ResponseCache.lock()
					{
						Cache.put(Ck, (std::time::Instant::now(), body.clone()));
					}
					let mut Builder = Response::builder().status(Status);
					Builder = CopyUpstreamHeaders(Builder, &UpstreamHeaders);
					return Builder
						.header("Content-Type", "application/json; charset=utf-8")
						.header("X-Aphrodite-Compressed", "true")
						.header("X-Aphrodite-Cache", "MISS")
						.header("X-Aphrodite-Fill-Pct", {
							let v = State.FillPct.load(Ordering::Relaxed) as f64 / 100.0;
							if v.is_finite() { format!("{:.1}", v) } else { "0.0".to_string() }
						})
						.body(Body::from(body))
						.unwrap();
				}
			}

			if State.Dev {
				let Elapsed = T0.elapsed();
				let BodyPreview = if ResponseBody.len() > 500 {
					let s = std::str::from_utf8(&ResponseBody).unwrap_or("?");
					let Preview:String = s.char_indices().take_while(|(i, _)| *i < 200).map(|(_, c)| c).collect();
					format!("{}... ({} total)", Preview, ResponseBody.len())
				} else {
					std::str::from_utf8(&ResponseBody).unwrap_or("?").to_string()
				};
				tracing::info!(
					ID = %RequestIDShort,
					Status = %Status,
					ResponseLength = ResponseBody.len(),
					ElapsedMs = Elapsed.as_millis(),
					body = %BodyPreview,
					"<<< RES"
				);
			}
			// Use already-extracted content_type (fetched before response.bytes())
			State.RecordLatency(T0.elapsed());
			State.RecordRequest(
				RequestIDShort,
				Method.as_str(),
				path,
				Status.as_u16(),
				false,
				T0.elapsed().as_millis(),
			);
			// Store raw response in LLM cache if applicable - success only (F2, see
			// the compressed-path cache write above for the full rationale).
			if let Some(Ck) = CacheKey
				&& Status.is_success()
				&& ResponseBody.len() <= ResponseCacheMaximumBodyBytes
				&& let Ok(mut Cache) = State.ResponseCache.lock()
			{
				Cache.put(Ck, (std::time::Instant::now(), ResponseBody.to_vec()));
			}
			let mut Builder = Response::builder().status(Status);
			Builder = CopyUpstreamHeaders(Builder, &UpstreamHeaders);
			Builder = Builder.header("X-Aphrodite-Cache", "MISS");
			Builder = Builder.header("X-Aphrodite-Fill-Pct", {
				let v = State.FillPct.load(Ordering::Relaxed) as f64 / 100.0;
				if v.is_finite() { format!("{:.1}", v) } else { "0.0".to_string() }
			});
			if let Some(Ct) = ContentType {
				Builder = Builder.header("Content-Type", Ct);
			}
			Builder.body(Body::from(ResponseBody)).unwrap()
		},
		Err(e) => {
			if FinalErrorWasTimeout {
				State.UpstreamTimeouts.fetch_add(1, Ordering::Relaxed);
			} else {
				State.UpstreamConnectErrors.fetch_add(1, Ordering::Relaxed);
			}
			State.RecordLatency(T0.elapsed());
			State.RecordRequest(RequestIDShort, Method.as_str(), path, 502, false, T0.elapsed().as_millis());
			State.RecordError(format!("upstream: {}", e));
			if State.Dev {
				tracing::error!(
					ID = %RequestIDShort,
					error = %e,
					ElapsedMs = T0.elapsed().as_millis(),
					"<<< ERR"
				);
			}
			// 02-F8: same rule as the body-read error above - `reqwest::Error`'s
			// `Display` can embed the upstream URL and, in some TLS/DNS
			// failure modes, host/proxy details; a caller shouldn't be able
			// to fingerprint `state.api_url` through this response.
			(
				StatusCode::BAD_GATEWAY,
				Json(serde_json::json!({"error": "upstream request failed"})),
			)
				.into_response()
		},
	}
}
