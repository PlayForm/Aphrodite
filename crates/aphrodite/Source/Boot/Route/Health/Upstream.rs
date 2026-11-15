//! `/health/upstream` - a cached probe of the upstream API.

use std::{net::SocketAddr, sync::Arc};

use aphrodite::Proxy::State::Struct::Struct as AppState;
use axum::{
	Router,
	extract::ConnectInfo,
	http::StatusCode,
	response::{IntoResponse, Json},
	routing::get,
};

/// `/health/upstream` - does the configured upstream answer `/models`?
///
/// Restricted, unlike its sibling: the probe spends the operator's upstream
/// rate-limit quota, so it repeats the loopback check itself in addition to the
/// layer the restricted router already applies. The 60-second cache exists for
/// the same reason - an uncached probe here is the same quota-burning cost class
/// the sibling `/health` check already fixed.
pub fn Route(State:&Arc<AppState>) -> Router<Arc<AppState>> {
	Router::new().route(
		"/health/upstream",
		get({
			let s = State.clone();
			move |ConnectInfo(Addr):ConnectInfo<SocketAddr>| {
				let s = s.clone();
				async move {
					if !Addr.ip().is_loopback() {
						return (
							StatusCode::FORBIDDEN,
							Json(serde_json::json!({
								"error": "only loopback clients allowed"
							})),
						)
							.into_response();
					}
					// F19: 60s TTL cache - see `UpstreamHealthCache`'s doc
					// comment for why an uncached probe here is the same
					// rate-limit-quota-burning cost class as the health
					// check this endpoint's sibling `/health` already fixed.
					const Ttl:std::time::Duration = std::time::Duration::from_secs(60);
					if let Some((Okay, At)) = *s.UpstreamHealthCache.lock().unwrap_or_else(|e| e.into_inner())
						&& At.elapsed() < Ttl
					{
						return Json(serde_json::json!({"upstream": Okay, "cached": true})).into_response();
					}
					let Okay = s
						.Client
						.get(format!("{}/models", s.APIURL.trim_end_matches('/')))
						.header("Authorization", format!("Bearer {}", s.APIKey.Expose()))
						.timeout(std::time::Duration::from_secs(5))
						.send()
						.await
						.map(|r| r.status().is_success())
						.unwrap_or(false);
					if let Ok(mut Cache) = s.UpstreamHealthCache.lock() {
						*Cache = Some((Okay, std::time::Instant::now()));
					}
					Json(serde_json::json!({"upstream": Okay, "cached": false})).into_response()
				}
			}
		}),
	)
}
