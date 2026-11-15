//! The `/ccr/create` handler: compress, store, report, notify.
//!
//! Stores through the same threshold and inline-CCR rules the proxy uses, so an
//! externally created entry behaves exactly like an intercepted one. Emits the
//! webhook notification when a new entry lands.

use std::{
	sync::{Arc, atomic::Ordering},
	time::Duration,
};

use axum::{
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Json},
};
use bytes::Bytes;
use headroom_core::ccr::compute_key;

use crate::Proxy::{
	CCR::{
		Create::{Request::Struct::Struct as CCRCreateRequest, Response::Struct::Struct as CCRCreateResponse},
		Notification::Struct::Struct as CCRNotification,
	},
	State::{CCR::Put::Fn::Fn as CCRPut, Compress::Fn::Fn as EstimateCompressedSize, Struct::Struct as AppState},
};

/// `POST /ccr/create` - stores content directly into the CCR backend,
/// bypassing the Chat Completions compression path. Accepts either a JSON
/// [`CCRCreateRequest`] body or a raw octet-stream (treated as the content
/// itself, hashed for the key). Fires the `NotifyURL` webhook on success
/// if configured.
pub async fn Fn(State(state):State<Arc<AppState>>, Headers:axum::http::HeaderMap, body:Bytes) -> impl IntoResponse {
	let ContentType = Headers.get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");

	// Support both JSON and raw octet-stream bodies
	match ContentType.contains("json") {
		true => {
			// Parse as JSON CCRCreateRequest
			match serde_json::from_slice::<CCRCreateRequest>(&body) {
				Ok(Request) => {
					let OriginalSize = Request.Content.len();
					let hash = Request.Key.unwrap_or_else(|| compute_key(Request.Content.as_bytes()));

					// F14: report unavailability instead of a fabricated success -
					// previously this endpoint returned a hash + savings ratio even
					// when `state.ccr` was `None` (e.g. token mode with
					// `--no-ccr-marker`), so nothing was ever stored and every
					// later `/retrieve` of that hash 404s. Mirrors
					// `HandleCCRDelete`'s existing `None` branch.
					let ccr = match &state.ccr {
						Some(ccr) => ccr,
						None => {
							return (
								StatusCode::SERVICE_UNAVAILABLE,
								Json(serde_json::json!({"error": "CCR not enabled"})),
							)
								.into_response();
						},
					};

					match !CCRPut(ccr, &hash, &Request.Content).await {
						true => {
							return (
								StatusCode::INTERNAL_SERVER_ERROR,
								Json(serde_json::json!({"error": "failed to store content in CCR backend"})),
							)
								.into_response();
						},
						false => {},
					}

					state.CCRCreated.fetch_add(1, Ordering::Relaxed);
					// Unlike the chat-completion/tool-relay paths, this
					// endpoint's documented wire contract IS the bare hash
					// (see `CompressedSize`/`MarkerSize` below and
					// `TestCCRCreateResponseSerdeShape`) - no marker is
					// rendered here, so `hash.len()` is the correct
					// subtractee, not an approximation (report 05 F5).
					state
						.TokensSaved
						.fetch_add(OriginalSize.saturating_sub(hash.len()) as u64, Ordering::Relaxed);
					state.RequestsCompressed.fetch_add(1, Ordering::Relaxed);

					// Update the compression ratio EMA so that /stats
					// reflects the compressibility of content flowing
					// through /ccr/create.
					//
					// Use a byte-entropy based estimate for the effective
					// compressed size, rather than the rendered marker
					// length (which is ~constant regardless of content
					// compressibility).  The estimate models a simplified
					// dictionary compressor: we count unique 3-byte
					// trigrams in the first 4096 bytes and use that count
					// (raised gently) as the compressed payout.  Highly
					// repetitive content (few unique trigrams) yields a
					// high ratio; random-looking content yields a low
					// ratio.
					let Estimate = EstimateCompressedSize(&Request.Content);
					state.UpdateCompressionRatio(OriginalSize, Estimate);

					match &state.NotifyURL {
						Some(NotifyURL) => {
							let Notification = CCRNotification {
								Event:"ccr_created".into(),
								hash:hash.clone(),
								CreatedAt:std::time::SystemTime::now()
									.duration_since(std::time::UNIX_EPOCH)
									.unwrap_or_default()
									.as_secs(),
								Ttl:Request.TtlSeconds.unwrap_or(3600),
								Tags:Request.Tags.unwrap_or_default(),
							};
							let Tracker = state.TaskTracker.clone();
							let Client = state.Client.clone();
							let url = NotifyURL.clone();
							let Key = state.NotifyKey.clone();
							let StateClone = state.clone();
							Tracker.spawn(async move {
								let mut Request = Client.post(&url).json(&Notification);

								match &Key {
									Some(k) => {
										Request = Request.header("Authorization", format!("Bearer {k}"));
									},
									None => {},
								}

								match Request.timeout(Duration::from_secs(5)).send().await {
									Ok(r) if r.status().is_success() => {
										StateClone.NotifySuccess.fetch_add(1, Ordering::Relaxed);
									},
									_ => {
										StateClone.NotifyFailure.fetch_add(1, Ordering::Relaxed);
									},
								}
							});
						},
						None => {},
					}

					let CompressedSize = hash.len();
					Json(CCRCreateResponse {
						hash,
						TokenSavingsRatio:match OriginalSize {
							0 => 1.0,
							_ => OriginalSize as f64 / CompressedSize.max(1) as f64,
						},
						OriginalSize,
						CompressedSize,
						MarkerSize:CompressedSize,
					})
					.into_response()
				},
				Err(e) => {
					(
						StatusCode::BAD_REQUEST,
						Json(serde_json::json!({"error": format!("invalid JSON: {}", e)})),
					)
						.into_response()
				},
			}
		},
		false => {
			// Treat raw body as content directly
			let Content = match String::from_utf8(body.to_vec()) {
				Ok(c) => c,
				Err(_) => {
					return (
						StatusCode::BAD_REQUEST,
						Json(serde_json::json!({"error": "invalid UTF-8 in body"})),
					)
						.into_response();
				},
			};
			let OriginalSize = Content.len();
			let hash = compute_key(Content.as_bytes());

			// F14/F4: same rules as the JSON-body branch above - 503 when CCR
			// isn't enabled, 500 (not a fabricated success) when the store write
			// itself fails.
			let ccr = match &state.ccr {
				Some(ccr) => ccr,
				None => {
					return (
						StatusCode::SERVICE_UNAVAILABLE,
						Json(serde_json::json!({"error": "CCR not enabled"})),
					)
						.into_response();
				},
			};

			match !CCRPut(ccr, &hash, &Content).await {
				true => {
					return (
						StatusCode::INTERNAL_SERVER_ERROR,
						Json(serde_json::json!({"error": "failed to store content in CCR backend"})),
					)
						.into_response();
				},
				false => {},
			}

			state.CCRCreated.fetch_add(1, Ordering::Relaxed);
			state.RequestsCompressed.fetch_add(1, Ordering::Relaxed);
			// See the JSON-body branch above: this endpoint's wire contract
			// IS the bare hash, so `hash.len()` is the correct subtractee.
			state
				.TokensSaved
				.fetch_add(OriginalSize.saturating_sub(hash.len()) as u64, Ordering::Relaxed);

			let CompressedSize = hash.len();
			Json(CCRCreateResponse {
				hash,
				TokenSavingsRatio:match OriginalSize {
					0 => 1.0,
					_ => OriginalSize as f64 / CompressedSize.max(1) as f64,
				},
				OriginalSize,
				CompressedSize,
				MarkerSize:CompressedSize,
			})
			.into_response()
		},
	}
}
