//! The catch-all proxy handler's body: one request, start to finish.
//!
//! Split from `Fn` only so the request span can be entered with `Instrument`
//! (an async fn cannot hold an `Entered` guard across its awaits). The sequence
//! itself is unchanged: count the request, log it in dev mode, build the
//! upstream URL and the cache key, then `Lookup`, `Forward`, `Respond`.

use std::sync::{Arc, atomic::Ordering};

use axum::{http::Method, response::IntoResponse};
use bytes::Bytes;

use super::super::{Forward, Lookup, Respond};
use crate::Proxy::{
	Body::Truncate::Fn::Fn as TruncateCharSafe,
	Cache::Key::Fn::Fn as CacheKeyFromBody,
	State::Struct::Struct as AppState,
	Threshold::Const::ChatCompletionsPath,
};

pub(crate) async fn Fn(
	state:Arc<AppState>,
	Method:Method,
	path:axum::extract::OriginalUri,
	Headers:axum::http::HeaderMap,
	body:Bytes,
) -> impl IntoResponse {
	state.RequestsTotal.fetch_add(1, Ordering::Relaxed);
	state.RequestBodyBytes.fetch_add(body.len() as u64, Ordering::Relaxed);
	let T0 = std::time::Instant::now();
	let RequestID = uuid::Uuid::new_v4().to_string();
	let RequestIDShort = &RequestID[..8];

	match state.Dev {
		true => {
			// Log incoming headers
			let mut HdrLog = String::new();

			Headers.iter().for_each(|(k, v)| {
				let Value = v.to_str().unwrap_or("?");

				match k.as_str().to_lowercase() != "authorization" {
					true => {
						HdrLog.push_str(&format!("  {}: {}", k.as_str(), TruncateCharSafe(Value, 80)));
					},
					false => {
						HdrLog.push_str("  authorization: ***");
					},
				}

				HdrLog.push('\n');
			});

			tracing::info!(
				ID = %RequestIDShort,
				Method = %Method,
				path = %path.path(),
				BodyLength = body.len(),
				Headers = %HdrLog,
				">>> REQ"
			);
		},
		false => {},
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
	let CacheKey = match IsChatCompletion {
		true => CacheKeyFromBody(&BodyVec, state.APIKey.Expose()),
		false => None,
	};
	match CacheKey {
		Some(Ck) => {
			match Lookup::Fn::Fn(&state, Some(Ck), RequestIDShort, &Method, path.path(), T0) {
				Some(Cached) => {
					return Cached;
				},
				None => {},
			}
		},
		None => {},
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
