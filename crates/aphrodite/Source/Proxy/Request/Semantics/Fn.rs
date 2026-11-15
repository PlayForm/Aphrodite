//! Interpret a request body, once.
//!
//! This is the only place a request body is parsed on the request path, and the
//! delivery mode and the cache key are both read off the same parsed value.
//!
//! That is the whole point of the leaf (D3): the streaming decision used to be
//! a byte scan for `"stream":true` anywhere in the body while the cache key
//! parsed the top-level field, so `{"model":"m","stream":false,"meta":{"stream":
//! true}}` was forwarded on the streaming client (no total timeout) and was
//! also cached - a buffered JSON body stored and replayed for a request the
//! proxy had classified as streaming. Now the top-level field decides both, and
//! a body carrying no parseable top-level `stream` is buffered.
//!
//! The unknown-body path is preserved exactly: a body that is not JSON, or JSON
//! of a shape that is not a chat completion, is `Opaque`/`Json`, buffered, and
//! uncacheable - it is forwarded unchanged.

use serde_json::Value;

use crate::Proxy::{
	Cache::Key::Fn::Fn as CacheKey,
	Request::{
		Body::Enum::Enum as RequestBody,
		Cacheability::Enum::Enum as RequestCacheability,
		Delivery::Enum::Enum as RequestDelivery,
		Route::{Classify::Fn::Fn as ClassifyRoute, Enum::Enum as RequestRoute},
		Semantics::Struct::Struct as RequestSemantics,
	},
};

/// Interpret `Raw` once: route, body kind, delivery mode and cacheability.
///
/// `Path` participates because cacheability is a route fact as well as a body
/// fact - the response cache only ever held Chat Completions answers - and
/// putting it here keeps that rule in one place instead of at the call site.
pub(crate) fn Fn(Path:&str, Raw:&[u8], APIKey:&str) -> RequestSemantics {
	let Route = ClassifyRoute(Path);
	let Parsed = serde_json::from_slice::<Value>(Raw).ok();
	let Delivery = Parsed
		.as_ref()
		.and_then(|Value| Value.get("stream"))
		.and_then(|Stream| Stream.as_bool())
		.filter(|Streaming| *Streaming)
		.map_or(RequestDelivery::Buffered, |_| RequestDelivery::Streaming);
	let Body = match Parsed {
		None => RequestBody::Opaque,
		Some(Value) => {
			match Value.get("model").and_then(|Model| Model.as_str()).is_some() && Value.get("messages").is_some() {
				true => RequestBody::ChatCompletion { Body:Value },
				false => RequestBody::Json,
			}
		},
	};
	let Cacheability = match (&Route, &Body, &Delivery) {
		(RequestRoute::ChatCompletions, RequestBody::ChatCompletion { Body: Value }, RequestDelivery::Buffered) => {
			CacheKey(Value, APIKey)
				.map_or(RequestCacheability::Uncacheable, |Key| RequestCacheability::Cacheable { Key })
		},
		_ => RequestCacheability::Uncacheable,
	};

	RequestSemantics { Route, Body, Delivery, Cacheability }
}
