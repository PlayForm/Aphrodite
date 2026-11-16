//! The progress report a live-container retrieve carries.

use serde::Serialize;

/// Progress status of a live container (pre-Rust port of `_hooks/live.py`), surfaced on every
/// retrieve of a container entry. `Count`/`Granularity` describe the stored container;
/// `Arrived` counts bytes served so far through partial retrieves (an approximation - it
/// advances by the bytes returned in each windowed response, including its `[lines ...]`
/// header) and `Complete` flips once the full content has been delivered - an `Exact` retrieve,
/// or a window whose unlimited tail reaches the container's end.
///
/// `Complete` is derived from `Arrived`, not carried beside it: a second flag that could
/// disagree with its own arithmetic has nothing to say.
///
/// The names are the response's own `snake_case` JSON keys, pinned per field, so the wire a
/// client already reads is unchanged.
#[derive(Debug, Clone, Serialize)]
pub struct Struct {
	/// `chunks`; number of chunks the container was stored in. `0` never reaches a caller - a
	/// degenerate envelope is served as a plain entry instead.
	#[serde(rename = "chunks")]
	pub Count:usize,
	/// `chunk_size`; target byte size of one chunk. `0` means the writer chose no target.
	#[serde(rename = "chunk_size")]
	pub Granularity:usize,
	/// `bytes_total`; total bytes the container holds. `0` is an empty container.
	#[serde(rename = "bytes_total")]
	pub Total:usize,
	/// `bytes_arrived`; bytes served so far across every retrieve of this container. `0` means
	/// nothing has been delivered yet.
	#[serde(rename = "bytes_arrived")]
	pub Arrived:usize,
	/// `complete`; `true` once the full content has been delivered.
	#[serde(rename = "complete")]
	pub Complete:bool,
}
