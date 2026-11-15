//! Accumulate an upstream body up to a hard byte cap.
//!
//! A single `response.bytes().await` can exhaust memory when the upstream
//! returns a huge body, so the read is chunked and aborted past the cap. Only
//! non-streaming responses come through here - SSE bodies are relayed chunk by
//! chunk and never buffered.

use futures::StreamExt;

/// Accumulate the full response body with a byte cap (report 02-T10).
/// Replaces the single unbounded `response.bytes().await` in the
/// non-streaming branch - returns a 502 error if the upstream exceeds
/// `MaximumBytes`, protecting the proxy's memory from a single huge response.
pub(crate) async fn Fn(response:reqwest::Response, MaximumBytes:usize) -> Result<bytes::Bytes, String> {
	let mut Buffer = Vec::new();
	let mut Stream = response.bytes_stream();
	while let Some(Chunk) = Stream.next().await {
		match Chunk {
			Ok(b) => {
				if Buffer.len() + b.len() > MaximumBytes {
					return Err(format!("response body exceeded {} MB limit", MaximumBytes / (1024 * 1024)));
				}
				Buffer.extend_from_slice(&b);
			},
			Err(e) => return Err(format!("body read: {}", e)),
		}
	}
	Ok(bytes::Bytes::from(Buffer))
}
