//! The catch-all proxy handler - the hot path of the whole binary.
//!
//! Entry point only: it counts the request, logs it in dev mode, builds the
//! upstream URL and the cache key, then hands off to the three stages -
//! `Lookup` (answer from cache), `Forward` (upstream with retry) and `Respond`
//! (stream, compress or fail). The stages are separate files because each one is
//! a distinct failure mode, and the handler itself stays readable.

use std::sync::{Arc, atomic::Ordering};

use axum::{extract::State, http::Method, response::IntoResponse};

use crate::Proxy::{
	Body::{Capped::Struct::Struct as CappedBody, Truncate::Fn::Fn as TruncateCharSafe},
	Cache::Key::Fn::Fn as CacheKeyFromBody,
	State::Struct::Struct as AppState,
	Threshold::Const::ChatCompletionsPath,
};
use super::{Forward, Lookup, Respond};

pub async fn Fn(
	State(state):State<Arc<AppState>>,
	Method:Method,
	path:axum::extract::OriginalUri,
	Headers:axum::http::HeaderMap,
	CappedBody(body):CappedBody,
) -> impl IntoResponse {
	state.RequestsTotal.fetch_add(1, Ordering::Relaxed);
	state.RequestBodyBytes.fetch_add(body.len() as u64, Ordering::Relaxed);
	let T0 = std::time::Instant::now();
	let RequestID = uuid::Uuid::new_v4().to_string();
	let RequestIDShort = &RequestID[..8];

	if state.Dev {
		// Log incoming headers
		let mut HdrLog = String::new();
		for (k, v) in Headers.iter() {
			let Value = v.to_str().unwrap_or("?");
			if k.as_str().to_lowercase() != "authorization" {
				HdrLog.push_str(&format!("  {}: {}", k.as_str(), TruncateCharSafe(Value, 80)));
			} else {
				HdrLog.push_str("  authorization: [REDACTED]");
			}
			HdrLog.push('\n');
		}
		tracing::info!(
			ID = %RequestIDShort,
			Method = %Method,
			path = %path.path(),
			BodyLength = body.len(),
			Headers = %HdrLog,
			">>> REQ"
		);
	}

	// F10: forward the query string too - `path.path()` excludes it, so any
	// OpenAI-compatible endpoint using query params (e.g. `GET
	// /v1/models?limit=5`) had it silently dropped by this catch-all.
	let DeepseekPathAndQuery = path
		.0
		.path_and_query()
		.map(|Pq| Pq.as_str())
		.unwrap_or_else(|| path.path())
		.trim_start_matches('/');
	let url = format!("{}/{}", state.APIURL.trim_end_matches('/'), DeepseekPathAndQuery);

	let IsChatCompletion = path.path().trim_start_matches('/') == ChatCompletionsPath.trim_start_matches('/');

	let BodyVec = body.to_vec();
	let CacheKey = if IsChatCompletion {
		CacheKeyFromBody(&BodyVec, state.APIKey.Expose())
	} else {
		None
	};
	if let Some(Ck) = CacheKey
		&& let Some(Cached) = Lookup::Fn::Fn(&state, Some(Ck), RequestIDShort, &Method, path.path(), T0)
	{
		return Cached;
	}
	// 02-F2: a `"stream": true` request goes out on `StreamClient` (no total
	// timeout), not the bounded `Client` - see `Forward`'s doc comment.
	let (UpstreamResult, FinalErrorWasTimeout) = Forward::Fn::Fn(&state, &Method, &url, &Headers, BodyVec).await;
	Respond::Fn::Fn(
		&state,
		&Method,
		path.path(),
		RequestIDShort,
		T0,
		&Headers,
		CacheKey,
		IsChatCompletion,
		UpstreamResult,
		FinalErrorWasTimeout,
	)
	.await
}
