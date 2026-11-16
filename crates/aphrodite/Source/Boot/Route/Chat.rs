//! The catch-all chat-completions proxy route.

use std::sync::Arc;

use aphrodite::{Proxy, Proxy::State::Struct::Struct as AppState};
use axum::{Router, extract::DefaultBodyLimit, routing::any};

/// `/{*path}` - everything no management route claimed goes upstream.
///
/// Carries its own 64 MB body limit, far larger than the management routes' 1 MB:
/// config advertises `[proxies] max_context` up to 1,000,000 tokens, and a chat request at
/// even 200k tokens is easily over 1 MB of JSON with tool schemas and history. A
/// shared 1 MB cap rejected large agent conversations with 413 before
/// `ProxyHandler` ever ran.
pub fn Route() -> Router<Arc<AppState>> {
	Router::new()
		.route("/{*path}", any(Proxy::Handle::Proxy::Fn::Fn))
		.layer(DefaultBodyLimit::max(64 * 1024 * 1024))
}
