//! Disposing of a caught panic payload.

use std::any::Any;

/// Turn a caught panic payload into the error the caller receives, without ever
/// running the payload's destructor.
///
/// ## The policy, and why it is this one
///
/// `catch_unwind` hands back the payload as a `Box<dyn Any + Send>`, and dropping
/// that box runs a destructor the panicking code chose. A destructor that panics
/// unwinds a second time - and the second unwind starts AFTER the catch that
/// produced the payload has returned, so nothing catches it. From an
/// `extern "C"` export that unwind aborts the host process. The ABI promises that
/// a recoverable panic becomes an error envelope, so the payload is **forgotten**
/// (`std::mem::forget`) and its destructor never runs.
///
/// The cost is a leak bounded by one payload per caught panic. That is the
/// documented trade for unconditional containment, not a zero-cost recovery: the
/// alternatives are dropping it (and losing the guarantee) or terminating the
/// process deliberately (and losing the response).
///
/// ## What is still reported
///
/// Before the payload is forgotten, a `&str` or `String` payload - the two shapes
/// `panic!` produces - is copied out and logged. Losing it entirely would leave an
/// operator with a bare "panicked" and nothing to act on, which is the whole
/// diagnosis a caught panic can give.
///
/// `catch_unwind` does not recover aborting panics and cannot make undefined
/// behaviour recoverable; this leaf narrows nothing about those.
pub(crate) fn Fn(Payload:Box<dyn Any + Send>, Message:&'static str) -> anyhow::Error {
	let Detail = Payload
		.downcast_ref::<&str>()
		.map(|Detail| (*Detail).to_string())
		.or_else(|| Payload.downcast_ref::<String>().cloned());

	std::mem::forget(Payload);

	Detail
		.into_iter()
		.for_each(|Detail| tracing::error!(detail = %Detail, "{Message}"));

	anyhow::anyhow!("{Message}")
}
