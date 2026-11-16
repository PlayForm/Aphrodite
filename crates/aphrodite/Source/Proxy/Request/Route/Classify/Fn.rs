//! Which route a path names.
//!
//! The leading slash is stripped on both sides because the router hands the
//! catch-all its path with the slash and the constant carries it too; comparing
//! them after the same normalisation is what kept `/v1/chat/completions` and
//! `v1/chat/completions` from being two different routes.

use std::cmp::Ordering;

use crate::Proxy::Request::Route::{Chat::Const::Path as Endpoint, Enum::Enum as ProxyRoute};

/// Classify a request path as the Chat Completions route or not.
///
/// `Path` is the inbound path as the catch-all router hands it over, leading
/// slash included; it is compared against the Chat Completions endpoint literal
/// (`Request::Route::Chat::Path`) after the same normalisation, so `/v1/chat/
/// completions` and `v1/chat/completions` are one route. `Other` is every path
/// that is not that endpoint - forwarded and returned untouched.
pub(crate) fn Fn(Path:&str) -> ProxyRoute {
	// Both sides are normalised the same way, and the comparison is the value.
	match Path.trim_start_matches('/').cmp(Endpoint.trim_start_matches('/')) {
		Ordering::Equal => ProxyRoute::ChatCompletions,
		Ordering::Less | Ordering::Greater => ProxyRoute::Other,
	}
}
