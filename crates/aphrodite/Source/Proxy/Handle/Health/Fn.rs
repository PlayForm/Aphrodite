//! The `/health` handler: liveness plus the numbers a supervisor needs.
//!
//! Reads only in-memory state and the CCR backend's O(1) `len()`, so it stays
//! answerable while the upstream is down - a health check that needs the
//! upstream reports the upstream's health, not this process's.

use std::sync::{Arc, atomic::Ordering};

use axum::{
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Json},
};

use crate::{Configuration::Proxy::Mode::Enum::Enum as ProxyMode, Proxy::State::Struct::Struct as AppState};

/// `GET /health` - local-only liveness check; does not call the upstream
/// API (see `/health/upstream` for that). Always returns 200 - capability
/// state (e.g. whether CCR is enabled) is conveyed via the JSON body
/// instead of the status code, since CCR is optional/opt-in.
pub async fn Fn(State(state):State<Arc<AppState>>) -> impl IntoResponse {
	let CCROk = state.ccr.is_some();

	(
		StatusCode::OK,
		Json(serde_json::json!({
			"status": "healthy",
			"ccr": CCROk,
			"mode": match state.Mode {
				ProxyMode::Cache => "cache",
				ProxyMode::Token => "token",
			},
			"version": env!("CARGO_PKG_VERSION"),
			// `FillPct` is a COMPRESSION HEADROOM number, not store
			// occupancy: it is `100 - compression_ratio_ema/20`, so a fresh
			// process reports exactly 90.0 with an empty store and drifts
			// toward 99 as the ratio approaches 1.0 (i.e. when little is
			// being compressed). An operator who reads it as "the store is
			// 90% full" gets a permanent false alarm - `store_entries` and
			// `/stats/db` are the store occupancy answers.
			"fill_pct": state.FillPct.load(Ordering::Relaxed) as f64 / 100.0,
			"fill_pct_kind": "compression_headroom",
			"store_entries": state.ccr.as_ref().map(|c| c.len()).unwrap_or(0),
		})),
	)
		.into_response()
}
