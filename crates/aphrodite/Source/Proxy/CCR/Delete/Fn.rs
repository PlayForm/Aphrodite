//! The `/ccr/delete` handler: remove one entry by hash.
//!
//! Reports the store's own verdict; a delete that the backend refused must not
//! read as success, or a caller believes a marker is gone while it resolves.

use std::sync::Arc;

use axum::{
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Json},
};

use crate::Proxy::State::{CCR::Delete::Fn::Fn as CCRDel, Struct::Struct as AppState};

/// `DELETE /ccr/:hash` - removes a single entry from the CCR backend.
/// Returns 404 if the hash wasn't present, 503 if no backend is configured.
pub async fn Fn(
	State(state):State<Arc<AppState>>,
	axum::extract::Path(hash):axum::extract::Path<String>,
) -> impl IntoResponse {
	match &state.ccr {
		Some(ccr) => {
			let Existed = CCRDel(ccr, &hash).await;

			match Existed {
				true => (StatusCode::OK, Json(serde_json::json!({"deleted": true, "hash": hash}))),
				false => {
					(
						StatusCode::NOT_FOUND,
						Json(serde_json::json!({"deleted": false, "hash": hash, "error": "not found"})),
					)
				},
			}
		},
		None => {
			(
				StatusCode::SERVICE_UNAVAILABLE,
				Json(serde_json::json!({"error": "CCR not enabled"})),
			)
		},
	}
}

// ── Config reload ──────────────────────────────────────────────────
