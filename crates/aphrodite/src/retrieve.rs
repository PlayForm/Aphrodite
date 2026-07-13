//! `/retrieve` endpoint - resolve CCR markers to original content.

use std::sync::Arc;

use axum::{
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};

use crate::proxy::{AppState, ccr_get};

/// Request body for `/retrieve`. `query`/`offset`/`limit` narrow down a large
/// stored payload instead of returning it in full.
#[derive(Debug, Deserialize)]
pub struct RetrieveRequest {
	pub hash:Option<String>,
	pub query:Option<String>,
	#[serde(default)]
	pub offset:usize,
	#[serde(default)]
	pub limit:usize,
}

/// Response body for `/retrieve`. `source` reports which backend served the
/// content (e.g. `"inline_ccr"`, `"ccr"`, `"none"`) for observability.
#[derive(Debug, Serialize)]
pub struct RetrieveResponse {
	pub found:bool,
	pub content:Option<String>,
	pub source:String,
	pub error:Option<String>,
}

/// Resolve a CCR marker hash back to its original content, optionally
/// filtered by `query` and paged by `offset`/`limit`. Checks the inline_ccr
/// store first, then falls back to the CCR backend.
pub async fn handle_retrieve(State(state):State<Arc<AppState>>, Json(req):Json<RetrieveRequest>) -> impl IntoResponse {
	let hash = match &req.hash {
		// Strip a `|type|size` marker-body suffix and surrounding whitespace
		// (report 05 F3) - an LLM sometimes echoes the full marker body back
		// as the hash argument instead of the bare hash, and this endpoint's
		// lookups (inline_ccr, CCR backend) are exact-match only.
		Some(h) => crate::marker::normalize_hash(h).to_string(),
		None => {
			return (
				StatusCode::BAD_REQUEST,
				Json(RetrieveResponse {
					found:false,
					content:None,
					source:"none".into(),
					error:Some("`hash` required".into()),
				}),
			)
				.into_response();
		},
	};

	// Check inline_ccr first (lock dropped before any .await to avoid !Send
	// MutexGuard)
	let mut content = {
		let inline_hit = state.inline_ccr.lock().ok().and_then(|mut map| map.get(&hash).cloned());
		if let Some(cached) = inline_hit {
			state.inline_ccr_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
			state.ccr_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
			cached
		} else {
			state.inline_ccr_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
			// Fallback to CCR backend
			match &state.ccr {
				Some(ccr) => {
					match ccr_get(ccr, &hash).await {
						Some(c) => {
							state.ccr_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
							c
						},
						None => {
							state.ccr_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
							return (
								StatusCode::NOT_FOUND,
								Json(RetrieveResponse {
									found:false,
									content:None,
									source:"none".into(),
									error:Some(format!("CCR entry not found: {}", hash)),
								}),
							)
								.into_response();
						},
					}
				},
				None => {
					state.ccr_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
					return (
						StatusCode::NOT_FOUND,
						Json(RetrieveResponse {
							found:false,
							content:None,
							source:"none".into(),
							error:Some(format!("CCR entry not found: {}", hash)),
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
	// and a real zstd frame is essentially never valid UTF-8, `content` here
	// could never legally contain the zstd magic bytes this branch checked
	// for. Removed rather than "fixed" - see `.plans/05-compression-pipeline.md`
	// T12 and `docs/ccr/lifecycle.md`'s retrieve-flow section.

	// Apply query filter, then pagination
	content = filter_content(&content, req.query.as_deref());
	content = match paginate(&content, req.offset, req.limit) {
		Ok(paged) => paged,
		Err(err) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(RetrieveResponse { found:false, content:Some(err), source:"ccr".into(), error:None }),
			)
				.into_response();
		},
	};

	Json(RetrieveResponse { found:true, content:Some(content), source:"ccr".into(), error:None }).into_response()
}

/// Keep only lines of `content` matching `query` (case-insensitive substring),
/// or return `content` unchanged if `query` is `None`/empty.
fn filter_content(content:&str, query:Option<&str>) -> String {
	match query {
		Some(q) if !q.is_empty() => {
			// Truncate FIRST (char-boundary-safe, not a raw byte slice), then
			// derive q_lower from the truncated query - previously q_lower
			// came from the untruncated q while only the *display* string
			// was capped, so the 512-char cap never actually bounded the
			// filter itself (report 05 F2).
			let q = crate::struct_extract::floor_boundary(q, 512);
			let q_lower = q.to_ascii_lowercase();
			let filtered:Vec<&str> = content
				.lines()
				.filter(|line| line.to_ascii_lowercase().contains(&q_lower))
				.collect();
			if filtered.is_empty() {
				format!("[no lines matching {:?} in {} lines]", q, content.lines().count())
			} else {
				filtered.join("\n")
			}
		},
		_ => content.to_string(),
	}
}

/// Slice `content` to the requested line window.
///
/// `limit == 0` means "unlimited" (clamped to 10_000 lines for safety).
/// Returns `Err(message)` when `offset` is at or past the end of the
/// document (mirrors the previous inline `BAD_REQUEST` behavior in
/// [`handle_retrieve`]). A `[lines a-b/total]` header is prepended only
/// when the window doesn't cover the whole document.
fn paginate(content:&str, offset:usize, limit:usize) -> Result<String, String> {
	// Clamp limit: 0 = unlimited, max 10_000 lines for safety
	let limit = if limit == 0 { 10_000 } else { limit.min(10_000) };
	let lines:Vec<&str> = content.lines().collect();
	let total = lines.len();
	if offset >= total {
		return Err(format!("[offset {} out of range; document has {} lines]", offset, total));
	}
	let start = offset.min(total);
	let end = (start + limit).min(total);
	let mut windowed = lines[start..end].join("\n");
	if start > 0 || end < total {
		windowed = format!("[lines {}-{}/{}]\n{}", start + 1, end, total, windowed);
	}
	Ok(windowed)
}

#[cfg(test)]
mod tests {
	use super::*;

	// ── T5 (F3): handle_retrieve must normalize the `hash` argument the
	// same way `resolve_one` already does - strip a `|type|size` marker-body
	// suffix an LLM might echo back, and trim surrounding whitespace. Seeds
	// `inline_ccr` directly (it's checked first, before any CCR backend
	// round-trip) via the crate's shared `AppState` test fixture instead of
	// duplicating its ~47-field literal.
	#[test]
	fn test_handle_retrieve_normalizes_pipe_suffixed_and_whitespace_hash() {
		let state = crate::proxy::tests::test_state_with_ccr();
		{
			let mut map = state.inline_ccr.lock().unwrap();
			map.put("abc123".to_string(), "<the real content>".to_string());
		}
		let state = Arc::new(state);

		async fn body_of(resp:axum::response::Response) -> serde_json::Value {
			let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
			serde_json::from_slice(&bytes).unwrap()
		}

		tokio::runtime::Runtime::new().unwrap().block_on(async {
			for hash_arg in ["abc123", "abc123|tool|1024", "  abc123  "] {
				let resp = handle_retrieve(
					State(state.clone()),
					Json(RetrieveRequest { hash:Some(hash_arg.to_string()), query:None, offset:0, limit:0 }),
				)
				.await
				.into_response();
				let body = body_of(resp).await;
				assert_eq!(body["found"], true, "hash arg {hash_arg:?} must resolve: {body:?}");
				assert_eq!(body["content"], "<the real content>");
			}
		});
	}

	// ── T9: paginate ──────────────────────────────────────────────
	#[test]
	fn test_paginate_offset_zero_full_document_no_header() {
		let content = "a\nb\nc";
		let result = paginate(content, 0, 0).unwrap();
		assert_eq!(result, "a\nb\nc");
	}

	#[test]
	fn test_paginate_offset_at_or_past_total_is_error() {
		let content = "a\nb\nc"; // 3 lines
		assert!(paginate(content, 3, 0).is_err());
		assert!(paginate(content, 10, 0).is_err());
		let err = paginate(content, 3, 0).unwrap_err();
		assert!(err.contains("out of range"));
		assert!(err.contains("3 lines"));
	}

	#[test]
	fn test_paginate_limit_zero_clamps_to_10000() {
		let content = (0..5).map(|i| i.to_string()).collect::<Vec<_>>().join("\n");
		let result = paginate(&content, 0, 0).unwrap();
		// All 5 lines returned, no windowing header since the whole doc fits.
		assert_eq!(result, content);
	}

	#[test]
	fn test_paginate_window_header_format() {
		let content = "a\nb\nc\nd\ne"; // 5 lines
		let result = paginate(content, 1, 2).unwrap();
		assert_eq!(result, "[lines 2-3/5]\nb\nc");
	}

	#[test]
	fn test_paginate_limit_clamped_to_10000_max() {
		let content = (0..5).map(|i| i.to_string()).collect::<Vec<_>>().join("\n");
		// A huge limit must behave the same as 0 (unlimited, capped at 10_000).
		let result = paginate(&content, 0, 999_999).unwrap();
		assert_eq!(result, content);
	}

	// ── T9: filter_content ────────────────────────────────────────
	#[test]
	fn test_filter_content_no_query_returns_unchanged() {
		assert_eq!(filter_content("a\nb\nc", None), "a\nb\nc");
		assert_eq!(filter_content("a\nb\nc", Some("")), "a\nb\nc");
	}

	#[test]
	fn test_filter_content_match() {
		let content = "alpha\nbeta error\ngamma\n";
		let result = filter_content(content, Some("error"));
		assert_eq!(result, "beta error");
	}

	#[test]
	fn test_filter_content_no_match_message() {
		let content = "alpha\nbeta\n";
		let result = filter_content(content, Some("nonexistent"));
		assert!(result.contains("no lines matching"));
		assert!(result.contains("2 lines"));
	}

	#[test]
	fn test_filter_content_case_insensitive() {
		let content = "ALPHA line\nbeta line\n";
		let result = filter_content(content, Some("alpha"));
		assert_eq!(result, "ALPHA line");
	}

	#[test]
	fn test_filter_content_query_over_512_chars_is_truncated() {
		let long_query = "x".repeat(600);
		let content = "some line with lots of x's\n";
		// Must not panic on a query longer than the content; truncates to 512 chars.
		let result = filter_content(content, Some(&long_query));
		assert!(result.contains("no lines matching"));
	}
}
