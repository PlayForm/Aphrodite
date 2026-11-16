//! `/health/upstream` - a cached probe of the upstream API.

use std::{
	net::SocketAddr,
	sync::Arc,
	time::{Duration, Instant},
};

use aphrodite::Proxy::State::Struct::Struct as AppState;
use axum::{
	Router,
	extract::ConnectInfo,
	http::StatusCode,
	response::{IntoResponse, Json, Response},
	routing::get,
};

use super::Peer::Enum::Enum as Peer;

/// How long one probe's answer is reused.
///
/// F19: 60s TTL cache - see `UpstreamHealthCache`'s doc comment for why an uncached
/// probe here is the same rate-limit-quota-burning cost class as the health check
/// this endpoint's sibling `/health` already fixed.
const Ttl:Duration = Duration::from_secs(60);

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
					match Peer::Of(&Addr) {
						Peer::Local => Answer(&s).await,
						Peer::Remote => Refusal(),
					}
				}
			}
		}),
	)
}

/// The answer for a peer that is not the machine itself: 403, and the upstream is
/// never asked.
fn Refusal() -> Response {
	(
		StatusCode::FORBIDDEN,
		Json(serde_json::json!({ "error": "only loopback clients allowed" })),
	)
		.into_response()
}

/// The cached answer when it is still fresh, the upstream when it is not.
///
/// The lock guard is dropped before either arm runs, so the cache write below
/// re-locks a free mutex instead of deadlocking on one held by the scrutinee.
async fn Answer(s:&Arc<AppState>) -> Response {
	let Cached = *s.UpstreamHealthCache.lock().unwrap_or_else(|e| e.into_inner());

	match Cached.filter(Fresh) {
		Some((Okay, _)) => Json(serde_json::json!({ "upstream": Okay, "cached": true })).into_response(),
		None => Probe(s).await,
	}
}

/// A cache entry still inside its TTL.
fn Fresh((_, At):&(bool, Instant)) -> bool { At.elapsed() < Ttl }

/// Ask the upstream, record what it said, and report it as uncached.
async fn Probe(s:&Arc<AppState>) -> Response {
	let Okay = s
		.Client
		.get(format!("{}/models", s.APIURL.trim_end_matches('/')))
		.header("Authorization", format!("Bearer {}", s.APIKey.Expose()))
		.timeout(Duration::from_secs(5))
		.send()
		.await
		.map(|r| r.status().is_success())
		.unwrap_or(false);

	// A poisoned cache is not worth failing a health probe over: the answer is
	// reported either way, and the next probe re-locks it. Recording is the effect
	// a successful lock carries, so the `Result` is the gate rather than a test.
	s.UpstreamHealthCache
		.lock()
		.ok()
		.into_iter()
		.for_each(|mut Cache| *Cache = Some((Okay, Instant::now())));

	Json(serde_json::json!({ "upstream": Okay, "cached": false })).into_response()
}
