//! The pure retry decision.
//!
//! Policy, attempt number, error and a jitter sample in; a verdict out. No
//! clock, no sleep, no `tracing`, no state: the whole safety policy can be read
//! and tested as a function, and the interpreter that sleeps is separate.
//!
//! F17: only connect-phase failures retry - the request never left, so resending
//! is safe. A post-send timeout may have been accepted by the upstream, and
//! every other transport failure fails fast on the first attempt.

use std::time::Duration;

use crate::Proxy::Handle::Proxy::Forward::{
	Decision::Enum::Enum as RetryDecision,
	Policy::Struct::Struct as ForwardPolicy,
};

/// Decide whether to retry, and how long to wait.
///
/// `Jitter` is a uniform sample in `[0, 1)`; the policy scales it into
/// `[JitterFloor, JitterFloor + JitterSpan)`. Taking it as an argument keeps
/// this function deterministic for a given input, which is what makes the
/// bounds checkable.
pub(crate) fn Fn(Policy:&ForwardPolicy, Attempt:u32, Error:&reqwest::Error, Jitter:f64) -> RetryDecision {
	(Attempt < Policy.MaximumAttempts && Error.is_connect())
		.then(|| {
			let BaseMs = Policy.Backoff.saturating_mul(2u64.saturating_pow(Attempt.saturating_sub(1)));
			let Ms = (BaseMs as f64 * (Policy.JitterFloor + Jitter * Policy.JitterSpan)) as u64;

			Duration::from_millis(Ms)
		})
		.map_or(RetryDecision::Stop, RetryDecision::RetryAfter)
}
