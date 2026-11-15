//! Classify a transport error.

use crate::Proxy::Handle::Proxy::Forward::Failure::Kind::Enum::Enum as FailureKind;

/// Which class of failure this error is.
///
/// `is_timeout` is asked first because a connect attempt can also time out, and
/// the timeout is the more specific diagnosis for an operator.
pub(crate) fn Fn(Error:&reqwest::Error) -> FailureKind {
	Error
		.is_timeout()
		.then_some(FailureKind::Timeout)
		.or_else(|| Error.is_connect().then_some(FailureKind::Connect))
		.unwrap_or(FailureKind::Other)
}
