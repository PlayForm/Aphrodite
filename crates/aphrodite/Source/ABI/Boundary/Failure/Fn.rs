//! `Failure`: an `anyhow::Error` as the `{"error": ...}` envelope.

use super::super::Answer::Enum::Enum as Answer;

/// Build the `{"error": <msg>}` payload every failure path returns.
///
/// `{Error:#}` is the alternate display, which renders the whole context chain
/// on one line - the caller's only diagnosis, so truncating it to the outermost
/// message would hide the operation that actually failed.
///
/// This is the whole of the ABI's error vocabulary: `extern "C"` never returns
/// an `anyhow::Error`, and every export's failure leaves here as the same
/// envelope so a host has one branch to write.
pub(crate) fn Fn(Error:&anyhow::Error) -> Answer {
	Answer::Envelope(serde_json::json!({"error": format!("{Error:#}")}))
}
