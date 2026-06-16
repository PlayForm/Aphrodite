//! `/retrieve` endpoint - resolve CCR markers to original content.

use std::sync::Arc;

use axum::{
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};

use crate::proxy::{ccr_get, AppState};

#[derive(Debug, Deserialize)]
pub struct RetrieveRequest {
	pub hash: Option<String>,
	pub query: Option<String>,
	#[serde(default)]
	pub offset: usize,
	#[serde(default)]
	pub limit: usize,
}

#[derive(Debug, Serialize)]
pub struct RetrieveResponse {
	pub found: bool,
	pub content: Option<String>,
	pub source: String,
	pub error: Option<String>,
}

pub async fn handle_retrieve(
	State(state): State<Arc<AppState>>,
	Json(req): Json<RetrieveRequest>,
) -> impl IntoResponse {
	let hash = match &req.hash {
		Some(h) => h.clone(),
		None => {
			return (
				StatusCode::BAD_REQUEST,
				Json(RetrieveResponse {
					found: false,
					content: None,
					source: "none".into(),
					error: Some("`hash` required".into()),
				}),
			)
				.into_response();
		},
	};

	// Check inline_ccr first (lock dropped before any .await to avoid !Send MutexGuard)
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
				Some(ccr) => match ccr_get(ccr, &hash).await {
					Some(c) => {
						state.ccr_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
						c
					},
					None => {
						state.ccr_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
						return (
							StatusCode::NOT_FOUND,
							Json(RetrieveResponse {
								found: false,
								content: None,
								source: "none".into(),
								error: Some(format!("CCR entry not found: {}", hash)),
							}),
						)
							.into_response();
					},
				},
				None => {
					state.ccr_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
					return (
						StatusCode::NOT_FOUND,
						Json(RetrieveResponse {
							found: false,
							content: None,
							source: "none".into(),
							error: Some(format!("CCR entry not found: {}", hash)),
						}),
					)
						.into_response();
				},
			}
		}
	};

	// Decompress zstd if magic bytes are present
	if content.as_bytes().starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
		let compressed_len = content.len();
		match zstd::decode_all(content.as_bytes()) {
			Ok(decompressed) => {
				content = String::from_utf8_lossy(&decompressed).to_string();
				tracing::debug!("zstd-decompressed content, {}b -> {}b", compressed_len, decompressed.len());
			},
			Err(e) => {
				tracing::warn!("zstd decompress failed for hash content: {}", e);
				return (
					StatusCode::INTERNAL_SERVER_ERROR,
					Json(RetrieveResponse {
						found: false,
						content: None,
						source: "ccr".into(),
						error: Some("decompression failed".into()),
					}),
				)
					.into_response();
			},
		}
	}

	// Apply query filter, then pagination
	content = filter_content(&content, req.query.as_deref());
	// Clamp limit: 0 = unlimited, max 10_000 lines for safety
	let limit = if req.limit == 0 { 10_000 } else { req.limit.min(10_000) };
	let lines: Vec<&str> = content.lines().collect();
	let total = lines.len();
	if req.offset >= total {
		return (
			StatusCode::BAD_REQUEST,
			Json(RetrieveResponse {
				found: false,
				content: Some(format!("[offset {} out of range; document has {} lines]", req.offset, total)),
				source: "ccr".into(),
				error: None,
			}),
		)
			.into_response();
	}
	let start = req.offset.min(total);
	let end = (start + limit).min(total);
	content = lines[start..end].join("\n");
	if start > 0 || end < total {
		content = format!("[lines {}-{}/{}]\n{}", start + 1, end, total, content);
	}

	Json(RetrieveResponse { found: true, content: Some(content), source: "ccr".into(), error: None }).into_response()
}

fn filter_content(content: &str, query: Option<&str>) -> String {
	match query {
		Some(q) if !q.is_empty() => {
			let q_lower = q.to_ascii_lowercase();
			let q = if q.len() > 512 { &q[..512] } else { q };
			let filtered: Vec<&str> = content
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
