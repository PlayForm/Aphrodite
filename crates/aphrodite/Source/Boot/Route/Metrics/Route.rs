//! `/metrics` - the route itself.

use std::sync::Arc;

use aphrodite::Proxy::State::Struct::Struct as AppState;
use axum::{Router, http::StatusCode, routing::get};

use crate::Boot::Route::Metrics::Body;

/// `/metrics` - the statistics in Prometheus text format.
///
/// Restricted by the loopback + `Host` layer but exempt from the bearer gate:
/// a Prometheus scraper typically cannot be configured to send a token. It
/// exposes the same counters as `/stats`, which is NOT exempt - the difference is
/// that `/stats` also carries request history, and a scraper has no business
/// reading that.
pub fn Route(State:&Arc<AppState>) -> Router<Arc<AppState>> {
	Router::new().route(
		"/metrics",
		get({
			let Shared = State.clone();
			move || {
				async move {
					let Stats = Shared.StatsJSON();
					(
						StatusCode::OK,
						[(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
						Body::Fn::Fn(&Stats),
					)
				}
			}
		}),
	)
}
