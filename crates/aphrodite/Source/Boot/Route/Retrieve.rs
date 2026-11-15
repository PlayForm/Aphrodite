//! The CCR retrieval route.

use std::sync::Arc;

use aphrodite::{Proxy::State::Struct::Struct as AppState, Retrieve};
use axum::{Router, routing::post};

/// `/retrieve` - expand a marker back to its stored content.
///
/// Restricted, and the reason the bearer gate exists: it reads stored session
/// content, so a hostile local page that could reach it would be reading the
/// operator's conversation history.
pub fn Route() -> Router<Arc<AppState>> { Router::new().route("/retrieve", post(Retrieve::Handle::Fn::Fn)) }
