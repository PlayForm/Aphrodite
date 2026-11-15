//! The bidirectional tool-relay route.

use std::sync::Arc;

use aphrodite::Proxy::{Relay::Handle::Fn::Fn as HandleToolRelay, State::Struct::Struct as AppState};
use axum::{Router, routing::post};

/// `/tool/relay` - a tool call the plugin hands to a client and waits on.
///
/// Restricted: a relay request causes an outbound callback, so an unauthenticated
/// caller could make this proxy emit traffic on its behalf.
pub fn Route() -> Router<Arc<AppState>> { Router::new().route("/tool/relay", post(HandleToolRelay)) }
