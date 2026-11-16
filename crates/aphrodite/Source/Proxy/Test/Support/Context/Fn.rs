//! Build a `RequestContext` without an HTTP server.
//!
//! The stages below `Route` take one context value, so a unit test builds one
//! instead of threading six arguments - and the interpretation inside it is the
//! same call the handler makes, so a test can never exercise a classification the
//! real path would not produce.

use std::sync::Arc;

use axum::http::{HeaderMap, Method};

use crate::Proxy::{
	Request::{
		Context::Struct::Struct as RequestContext,
		Route::Chat::Const::Path,
		Semantics::Fn::Fn as InterpretRequest,
	},
	State::Struct::Struct as AppState,
};

/// A context for `Body` posted to the Chat Completions route.
pub(crate) fn Fn<'a>(
	State:&'a Arc<AppState>,
	Method:&'a Method,
	Headers:&'a HeaderMap,
	Body:&[u8],
) -> RequestContext<'a> {
	RequestContext {
		State,
		Method,
		Path,
		ID:"test",
		Started:std::time::Instant::now(),
		Headers,
		Semantics:InterpretRequest(Path, Body, State.APIKey.Expose()),
	}
}
