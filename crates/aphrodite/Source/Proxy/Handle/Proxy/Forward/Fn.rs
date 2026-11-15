//! Send the upstream request, with the connect-only retry policy.
//!
//! The body is a `Bytes` built once by the caller, so each attempt clones a
//! refcount rather than the payload (bug 18-P9: up to 4 copies of a 1MB body
//! under retry load). The client comes from the request's delivery mode: a
//! `stream` request goes out on the client with no total timeout, because
//! reqwest's client-level timeout bounds the whole request lifetime including
//! consuming the response body, and a completion streaming tokens past 300s had
//! its `bytes_stream()` killed mid-answer (02-F2).
//!
//! The loop is the effectful interpreter of `Decide`: it sends, asks the pure
//! decision what to do, sleeps and retries, or returns the typed failure. There
//! is no seeded terminal error and no boolean beside the result - `Stop` is a
//! value the loop returns on.

use bytes::Bytes;

use crate::Proxy::{
	Handle::Proxy::Forward::{
		Decide::Fn::Fn as DecideRetry,
		Decision::Enum::Enum as RetryDecision,
		Failure::{
			Build::Fn::Fn as ForwardFailure,
			Kind::Classify::Fn::Fn as ClassifyFailure,
			Struct::Struct as FailureStruct,
		},
		Policy::Const::Const as RetryPolicy,
		Send::Fn::Fn as SendOnce,
	},
	Request::{Context::Struct::Struct as RequestContext, Delivery::Enum::Enum as RequestDelivery},
};

/// Send the request, retrying connect-phase failures.
pub(crate) async fn Fn(Context:&RequestContext<'_>, Url:&str, Body:Bytes) -> Result<reqwest::Response, FailureStruct> {
	let Client = match Context.Semantics.Delivery {
		RequestDelivery::Streaming => &Context.State.StreamClient,
		RequestDelivery::Buffered => &Context.State.Client,
	};
	let mut Attempt = 1u32;

	loop {
		let Sent = SendOnce(
			Client,
			Context.Method,
			Url,
			Context.Headers,
			Context.State.APIKey.Expose(),
			Body.clone(),
		)
		.await;

		match Sent {
			Ok(Response) => return Ok(Response),
			Err(Error) => {
				match DecideRetry(&RetryPolicy, Attempt, &Error, rand::random::<f64>()) {
					RetryDecision::RetryAfter(Delay) => {
						tracing::warn!(
							Attempt,
							BackoffMs = Delay.as_millis() as u64,
							"upstream retry after connect error: {}",
							Error
						);
						tokio::time::sleep(Delay).await;
						Attempt = Attempt.saturating_add(1);
					},
					RetryDecision::Stop => {
						return Err(ForwardFailure(ClassifyFailure(&Error), Attempt, Error, Url));
					},
				}
			},
		}
	}
}
