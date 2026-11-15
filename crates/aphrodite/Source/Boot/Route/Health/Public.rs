//! `/health` - the one route outside every guard layer.

use std::sync::Arc;

use aphrodite::{Proxy, Proxy::State::Struct::Struct as AppState};
use axum::{Router, routing::get};

/// `/health` - a liveness answer with no credentials and no loopback requirement.
///
/// Merged into the app on its own, never into the restricted router: an external
/// load balancer has to be able to reach it from a non-loopback address, and it
/// must not be gated by a bearer token a probe cannot send.
pub fn Route() -> Router<Arc<AppState>> { Router::new().route("/health", get(Proxy::Handle::Health::Fn::Fn)) }
