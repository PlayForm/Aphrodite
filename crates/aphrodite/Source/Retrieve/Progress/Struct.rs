//! The progress report a live-container retrieve carries.

use serde::Serialize;

/// Progress status of a live container (pre-Rust port of `_hooks/live.py`),
/// surfaced on every retrieve of a container entry. `Chunks`/`ChunkSize`
/// describe the stored container; `BytesArrived` counts bytes served so
/// far through partial retrieves (an approximation - it advances by the
/// bytes returned in each windowed response, including its `[lines ...]`
/// header) and `Complete` flips once the full content has been delivered
/// (final resolve, `limit: 0`).
#[derive(Debug, Clone, Serialize)]
pub struct Struct {
	/// Number of chunks the container was stored in.
	pub Chunks:usize,
	/// Target byte size of one chunk.
	pub ChunkSize:usize,
	/// Total bytes the container holds.
	pub BytesTotal:usize,
	/// Bytes served so far across every retrieve of this container.
	pub BytesArrived:usize,
	/// `true` once the full content has been delivered.
	pub Complete:bool,
}
