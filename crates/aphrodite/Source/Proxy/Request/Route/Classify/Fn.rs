//! Which route a path names.
//!
//! The leading slash is stripped on both sides because the router hands the
//! catch-all its path with the slash and the constant carries it too; comparing
//! them after the same normalisation is what kept `/v1/chat/completions` and
//! `v1/chat/completions` from being two different routes.

use crate::Proxy::{Request::Route::Enum::Enum as ProxyRoute, Threshold::Const::ChatCompletionsPath};

/// Classify a request path as the Chat Completions route or not.
pub(crate) fn Fn(Path:&str) -> ProxyRoute {
	if Path.trim_start_matches('/') == ChatCompletionsPath.trim_start_matches('/') {
		ProxyRoute::ChatCompletions
	} else {
		ProxyRoute::Other
	}
}
