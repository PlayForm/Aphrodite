//! Which route a path names.
//!
//! The leading slash is stripped on both sides because the router hands the
//! catch-all its path with the slash and the constant carries it too; comparing
//! them after the same normalisation is what kept `/v1/chat/completions` and
//! `v1/chat/completions` from being two different routes.

use std::cmp::Ordering;

use crate::Proxy::{Request::Route::Enum::Enum as ProxyRoute, Threshold::Const::ChatCompletionsPath};

/// Classify a request path as the Chat Completions route or not.
pub(crate) fn Fn(Path:&str) -> ProxyRoute {
	// Both sides are normalised the same way, and the comparison is the value.
	match Path.trim_start_matches('/').cmp(ChatCompletionsPath.trim_start_matches('/')) {
		Ordering::Equal => ProxyRoute::ChatCompletions,
		Ordering::Less | Ordering::Greater => ProxyRoute::Other,
	}
}
