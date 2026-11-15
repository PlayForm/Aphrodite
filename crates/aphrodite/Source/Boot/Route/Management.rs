//! The management routes: identity, statistics, history and the root banner.

use std::sync::Arc;

use aphrodite::{Configuration::Proxy::Mode::Enum::Enum as ProxyMode, Proxy::State::Struct::Struct as AppState};
use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};

use crate::Boot::Route::Metrics;

/// The management group, minus `/metrics` (which has its own leaf).
///
/// Every route here is on the restricted router, so each one is behind the
/// loopback + `Host` check and the management bearer token. `/metrics` is the one
/// deliberate exemption from the bearer check, and it is documented at the gate
/// itself rather than here.
pub fn Route(State:&Arc<AppState>) -> Router<Arc<AppState>> {
	Router::new()
		.route("/version", get(|| async { env!("CARGO_PKG_VERSION") }))
		.route(
			"/stats",
			get({
				let s = State.clone();
				move || async move { Json(s.StatsJSON()) }
			}),
		)
		.route(
			"/stats/db",
			get({
				let s = State.clone();
				move || {
					async move {
						let Mode = match s.Mode {
							ProxyMode::Cache => "cache",
							ProxyMode::Token => "token",
						};
						match &s.ccr {
							Some(ccr) => {
								match ccr.stats_db() {
									Some(Stats) => Json(Stats).into_response(),
									None => {
										(
											StatusCode::OK,
											Json(serde_json::json!({
												"error": "stats_db not available for this backend",
												"mode": Mode,
											})),
										)
											.into_response()
									},
								}
							},
							None => {
								(
									StatusCode::OK,
									Json(serde_json::json!({
										"error": "CCR not enabled",
										"mode": Mode,
									})),
								)
									.into_response()
							},
						}
					}
				}
			}),
		)
		.route(
			"/history",
			get({
				let s = State.clone();
				move || async move { Json(s.RequestHistory.lock().map(|v| v.clone()).unwrap_or_default()) }
			}),
		)
		.route("/favicon.ico", get(|| async { StatusCode::NOT_FOUND }))
		.route("/robots.txt", get(|| async { "User-agent: *\nDisallow: /\n" }))
		.route(
			"/",
			get(|| {
				async {
					Json(serde_json::json!({
						"proxy": "aphrodite",
						"version": env!("CARGO_PKG_VERSION"),
						"git_hash": option_env!("APHRODITE_GIT_HASH"),
						"build_date": option_env!("APHRODITE_BUILD_DATE"),
						"target": option_env!("APHRODITE_TARGET"),
						"profile": option_env!("APHRODITE_PROFILE"),
					}))
				}
			}),
		)
		.merge(Metrics::Route(State))
}
