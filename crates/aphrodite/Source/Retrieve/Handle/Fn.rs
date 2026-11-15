//! The `/retrieve` handler: resolve, reassemble, filter, page, report progress.

use std::sync::Arc;

use axum::{
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Json},
};

use crate::{
	Proxy::State::{CCR::Get::Fn::Fn as CCRGet, Struct::Struct as AppState},
	Retrieve::{
		Filter::Fn::Fn as FilterContent,
		Live::Lookup::Fn::Fn as LookupLiveContainer,
		Page::Fn::Fn as Paginate,
		Progress::Struct::Struct as LiveProgress,
		Request::Struct::Struct as RetrieveRequest,
		Response::Struct::Struct as RetrieveResponse,
	},
};

/// Resolve a CCR marker hash back to its original content, optionally
/// filtered by `Query` and paged by `Offset`/`Limit`. Checks the inline_ccr
/// store first, then falls back to the CCR backend.
pub async fn Fn(State(state):State<Arc<AppState>>, Json(Request):Json<RetrieveRequest>) -> impl IntoResponse {
	let hash = match &Request.hash {
		// Strip a `|type|size` marker-body suffix and surrounding whitespace
		// (report 05 F3) - an LLM sometimes echoes the full marker body back
		// as the hash argument instead of the bare hash, and this endpoint's
		// lookups (inline_ccr, CCR backend) are exact-match only.
		Some(h) => crate::Marker::NormalizeHash(h).to_string(),
		None => {
			return (
				StatusCode::BAD_REQUEST,
				Json(RetrieveResponse {
					Found:false,
					Content:None,
					Source:"none".into(),
					error:Some("`hash` required".into()),
					Truncated:false,
					Live:None,
				}),
			)
				.into_response();
		},
	};

	// Check inline_ccr first (lock dropped before any .await to avoid !Send
	// MutexGuard)
	let Content = {
		// Fix 20 (inspection): the old `.lock().ok().and_then(...)` silently
		// converted a poisoned mutex into a cache miss, so every request fell
		// through to the CCR backend and callers saw confusing 404s. A
		// poisoned lock is a broken-process condition - surface it as an
		// explicit 500 instead of a miss. The guard is still dropped before
		// any `.await` (the `Ok` arm's closure ends before the fallback
		// backend path below).
		let InlineHit = match state.InlineCCR.lock() {
			Ok(mut Map) => Map.get(&hash).cloned(),
			Err(_) => {
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(RetrieveResponse {
						Found:false,
						Content:None,
						Source:"none".into(),
						error:Some("inline_ccr lock poisoned; cannot serve /retrieve".into()),
						Truncated:false,
						Live:None,
					}),
				)
					.into_response();
			},
		};
		if let Some(Cached) = InlineHit {
			state.InlineCCRHits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
			state.CCRHits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
			Cached
		} else {
			state.InlineCCRMisses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
			// Fallback to CCR backend
			match &state.ccr {
				Some(ccr) => {
					match CCRGet(ccr, &hash).await {
						Some(c) => {
							state.CCRHits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
							c
						},
						None => {
							state.CCRMisses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
							return (
								StatusCode::NOT_FOUND,
								Json(RetrieveResponse {
									Found:false,
									Content:None,
									Source:"none".into(),
									error:Some(format!("CCR entry not found: {}", hash)),
									Truncated:false,
									Live:None,
								}),
							)
								.into_response();
						},
					}
				},
				None => {
					state.CCRMisses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
					return (
						StatusCode::NOT_FOUND,
						Json(RetrieveResponse {
							Found:false,
							Content:None,
							Source:"none".into(),
							error:Some(format!("CCR entry not found: {}", hash)),
							Truncated:false,
							Live:None,
						}),
					)
						.into_response();
				},
			}
		}
	};

	// NOTE (report 05 F12): a zstd-magic-byte decompression branch used to
	// live here. It was unreachable dead code: `CcrStore::get` returns
	// `Option<String>` (vendor/headroom/crates/headroom-core/src/ccr/mod.rs)
	// and every backend implementation (in-memory, SQLite, Redis) stores and
	// returns the exact `String`/`&str` payload verbatim - none of them ever
	// zstd-encodes content. Since a Rust `String` is guaranteed valid UTF-8,
	// and a real zstd frame is essentially never valid UTF-8, `Content` here
	// could never legally contain the zstd magic bytes this branch checked
	// for. Removed rather than "fixed" - see `.plans/05-compression-pipeline.md`
	// T12 and `docs/ccr/lifecycle.md`'s retrieve-flow section.

	// ── Live container progression (pre-Rust port of _hooks/live.py) ──
	// When the resolved entry is a live container (its metadata envelope
	// sits in the inline store), serve the content from the container's own
	// chunks and surface progress: chunk count / bytes arrived on partial
	// retrieves, full content + complete status on the final resolve
	// (limit: 0). A missing or malformed envelope degrades to the plain
	// entry (warn + degrade, never panic).
	let LiveContainer = LookupLiveContainer(&state, &hash, &Content);
	let mut Content = LiveContainer
		.as_ref()
		.map(|(_Meta, Assembled)| Assembled.clone())
		.unwrap_or(Content);

	// Apply query filter, then pagination
	Content = FilterContent(&Content, Request.Query.as_deref());
	let (Content, Truncated) = match Paginate(&Content, Request.Offset, Request.Limit) {
		Ok(Paged) => Paged,
		Err(Error) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(RetrieveResponse {
					Found:false,
					Content:Some(Error),
					Source:"ccr".into(),
					error:None,
					Truncated:false,
					Live:None,
				}),
			)
				.into_response();
		},
	};

	// Advance + surface the container progress after pagination, so the
	// served window's byte count is known. A final resolve (limit: 0)
	// marks the container complete; a partial window advances
	// `BytesArrived` by the bytes served in this response. The envelope is
	// written back so later retrieves observe the accumulated progress.
	let Live = match LiveContainer {
		Some((meta, _)) => {
			let FinalResolve = Request.Limit == 0;
			let Served = if FinalResolve { meta.Size } else { Content.len().min(meta.Size) };
			let BytesArrived = meta.BytesArrived.saturating_add(Served).min(meta.Size);
			let Complete = FinalResolve || BytesArrived >= meta.Size;
			let Progress = LiveProgress {
				Chunks:meta.Chunks,
				ChunkSize:meta.ChunkSize,
				BytesTotal:meta.Size,
				BytesArrived,
				Complete,
			};
			// Best-effort write-back: a poisoned lock or serialization
			// hiccup degrades to no-writeback (progress stays as-is).
			if let Ok(mut Map) = state.InlineCCR.lock() {
				let mut Updated = meta;
				Updated.BytesArrived = BytesArrived;
				if let Ok(Encoded) = serde_json::to_string(&Updated) {
					Map.put(crate::Hook::Live::Key::Fn::LiveMetaKey(&hash), Encoded);
				}
			}
			Some(Progress)
		},
		None => None,
	};

	Json(RetrieveResponse {
		Found:true,
		Content:Some(Content),
		Source:"ccr".into(),
		error:None,
		Truncated,
		Live,
	})
	.into_response()
}
