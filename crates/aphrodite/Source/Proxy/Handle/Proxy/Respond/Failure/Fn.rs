//! Render a terminal upstream failure.
//!
//! The counter comes from the failure's own kind, so the metrics no longer have
//! to reconstruct "was this a timeout" from a boolean that travelled beside the
//! result. `Other` is a category this proxy did not previously have: every
//! non-timeout transport failure used to be counted as a connect error, which is
//! a behaviour change and is named as one in the handoff.
//!
//! The error string is recorded server-side only (02-F8) - `reqwest::Error`'s
//! `Display` can embed the upstream URL.

use std::sync::atomic::Ordering;

use axum::response::Response;

use crate::Proxy::{
	Handle::Proxy::{
		Forward::Failure::{Kind::Enum::Enum as FailureKind, Struct::Struct as FailureStruct},
		Respond::BadGateway::Fn::Fn as BadGateway,
	},
	Log::Dev::Fn::Fn as DevLog,
	Request::Context::Struct::Struct as RequestContext,
};

/// Count the failure, record it, and answer the client.
pub(crate) fn Fn(Context:&RequestContext<'_>, Failure:FailureStruct) -> Response {
	match Failure.Kind {
		FailureKind::Timeout => {
			Context.State.UpstreamTimeouts.fetch_add(1, Ordering::Relaxed);
		},
		FailureKind::Connect => {
			Context.State.ConnectErrors.fetch_add(1, Ordering::Relaxed);
		},
		FailureKind::Other => {
			Context.State.OtherErrors.fetch_add(1, Ordering::Relaxed);
		},
	}

	Context.State.RecordLatency(Context.Started.elapsed());
	Context.State.RecordRequest(
		Context.ID,
		Context.Method.as_str(),
		Context.Path,
		502,
		false,
		Context.Started.elapsed().as_millis(),
	);
	Context.State.RecordError(format!("upstream: {:#}", Failure.Source));

	DevLog(Context.State.Dev, || {
		tracing::error!(
			ID = %Context.ID,
			error = %Failure.Source,
			Attempts = Failure.Attempts,
			ElapsedMs = Context.Started.elapsed().as_millis(),
			"<<< ERR"
		)
	});

	BadGateway()
}
