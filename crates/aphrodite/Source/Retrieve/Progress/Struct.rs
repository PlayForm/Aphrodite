//! The progress report a live-container retrieve carries.

use serde::Serialize;

/// Progress status of a live container (pre-Rust port of `_hooks/live.py`), surfaced on every
/// retrieve of a container entry. `Chunks`/`ChunkSize` describe the stored container;
/// `BytesArrived` counts bytes served so far through partial retrieves (an approximation - it
/// advances by the bytes returned in each windowed response, including its `[lines ...]`
/// header) and `Complete` flips once the full content has been delivered - an `Exact` retrieve,
/// or a window whose unlimited tail reaches the container's end.
///
/// `Complete` is derived from `BytesArrived`, not carried beside it: a second flag that could
/// disagree with its own arithmetic has nothing to say.
#[derive(Debug, Clone, Serialize)]
pub struct Struct {
	/// Number of chunks the container was stored in.
	#[serde(rename = "chunks")]
	pub Chunks:usize,
	/// Target byte size of one chunk.
	#[serde(rename = "chunk_size")]
	pub ChunkSize:usize,
	/// Total bytes the container holds.
	#[serde(rename = "bytes_total")]
	pub BytesTotal:usize,
	/// Bytes served so far across every retrieve of this container.
	#[serde(rename = "bytes_arrived")]
	pub BytesArrived:usize,
	/// `true` once the full content has been delivered.
	#[serde(rename = "complete")]
	pub Complete:bool,
}
