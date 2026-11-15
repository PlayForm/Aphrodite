//! Accumulate an upstream body up to a hard byte cap.
//!
//! A single `response.bytes().await` can exhaust memory when the upstream
//! returns a huge body, so the read is chunked and aborted past the cap. Only
//! non-streaming responses come through here - SSE bodies are relayed chunk by
//! chunk and never buffered.

use anyhow::Context;
use futures::StreamExt;

/// Accumulate the full response body with a byte cap (report 02-T10).
/// Replaces the single unbounded `response.bytes().await` in the
/// non-streaming branch - returns an error if the upstream exceeds
/// `MaximumBytes`, protecting the proxy's memory from a single huge response.
///
/// A mid-stream read failure and an over-cap body are different errors: the
/// first names how much had arrived, the second names the cap. Both reach the
/// client as the same generic 502, but `/stats`'s `LastErrors` keeps the
/// distinction for whoever is diagnosing the upstream.
pub(crate) async fn Fn(response:reqwest::Response, MaximumBytes:usize) -> anyhow::Result<bytes::Bytes> {
	let mut Buffer = Vec::new();
	let mut Stream = response.bytes_stream();
	while let Some(Chunk) = Stream.next().await {
		let Chunk = Chunk.with_context(|| format!("read the upstream body ({} bytes so far)", Buffer.len()))?;

		if Buffer.len() + Chunk.len() > MaximumBytes {
			anyhow::bail!("response body exceeded {} MB limit", MaximumBytes / (1024 * 1024));
		}

		Buffer.extend_from_slice(&Chunk);
	}
	Ok(bytes::Bytes::from(Buffer))
}
