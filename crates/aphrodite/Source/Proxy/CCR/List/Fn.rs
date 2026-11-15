//! The `/ccr/list` handler: the store's own listing, straight through.
//!
//! Read-only and cheap by design - it reports what the backend holds, not what
//! this process has seen, so it stays correct after a restart.

use std::sync::Arc;

use axum::{
	extract::State,
	response::{IntoResponse, Json},
};

use crate::{
	Configuration::Proxy::Mode::Enum::Enum as ProxyMode,
	Proxy::State::{CCR::Length::Fn::Fn as CCRLength, Struct::Struct as AppState},
};

/// `GET /ccr/list` - reports entry count and backend kind for the active
/// CCR store (no listing of actual entries/hashes).
pub async fn Fn(State(state):State<Arc<AppState>>) -> impl IntoResponse {
	match &state.ccr {
		Some(ccr) => {
			let Entries = CCRLength(ccr).await;
			Json(serde_json::json!({
				"entries": Entries,
				"backend": match state.Mode {
					ProxyMode::Cache => "in_memory",
					ProxyMode::Token => "sqlite",
				},
				"mode": match state.Mode {
					ProxyMode::Cache => "cache",
					ProxyMode::Token => "token",
				},
			}))
		},
		None => Json(serde_json::json!({"entries": 0, "message": "CCR not enabled"})),
	}
}
