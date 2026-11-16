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
//!
//! The parsed value dies here. It answers three questions - delivery, kind,
//! identity - and is then dropped: the identity carries the canonical bytes the
//! cache compares, so nothing downstream needs the tree, and keeping it alive in
//! `Request::Context` only held the request's JSON for the request's lifetime.

use serde_json::Value;

use crate::Proxy::{
	Cache::Identity::Struct::Struct as CacheIdentity,
	Request::{
		Body::Enum::Enum as RequestBody,
		Cacheability::Enum::Enum as RequestCacheability,
		Delivery::Enum::Enum as RequestDelivery,
		Route::{Classify::Fn::Fn as ClassifyRoute, Enum::Enum as RequestRoute},
		Semantics::Struct::Struct as RequestSemantics,
	},
};

/// A Chat Completions request: a string `model` and a `messages` member.
fn IsChat(Value:&Value) -> bool {
	Value.get("model").and_then(|Model| Model.as_str()).is_some() && Value.get("messages").is_some()
}

/// Interpret `Raw` once: route, body kind, delivery mode and cacheability.
///
/// `Path` participates because cacheability is a route fact as well as a body
/// fact - the response cache only ever held Chat Completions answers - and
/// putting it here keeps that rule in one place instead of at the call site.
pub(crate) fn Fn(Path:&str, Raw:&[u8], APIKey:&str) -> RequestSemantics {
	let Route = ClassifyRoute(Path);
	let Parsed:Option<Value> = serde_json::from_slice(Raw).ok();
	let Delivery = Parsed
		.as_ref()
		.and_then(|Value| Value.get("stream"))
		.and_then(|Stream| Stream.as_bool())
		.filter(|Streaming| *Streaming)
		.map_or(RequestDelivery::Buffered, |_| RequestDelivery::Streaming);
	// One shape decision, made once from the parsed value; every arm below reads
	// it rather than re-testing the body.
	// `filter`'s predicate sees `&&Value` (the option holds `&Value`), so the
	// predicate derefs once to hand `IsChat` the value itself.
	let Chat = Parsed.as_ref().filter(|Value| IsChat(Value));

	// The identity is derived inside the arm that uses it, so a streamed request -
	// the common case for chat - never pays for canonicalising a body it will not
	// cache.
	let (Body, Cacheability) = match (Route, Chat, Delivery) {
		(RequestRoute::ChatCompletions, Some(Value), RequestDelivery::Buffered) => {
			(
				RequestBody::ChatCompletion,
				RequestCacheability::Cacheable { Identity:CacheIdentity::New(Value, APIKey) },
			)
		},
		(RequestRoute::ChatCompletions, Some(_), RequestDelivery::Streaming) => {
			(RequestBody::ChatCompletion, RequestCacheability::Uncacheable)
		},
		(_, Some(_), _) => (RequestBody::Json, RequestCacheability::Uncacheable),
		(_, None, _) => (RequestBody::Opaque, RequestCacheability::Uncacheable),
	};

	RequestSemantics { Route, Body, Delivery, Cacheability }
}
