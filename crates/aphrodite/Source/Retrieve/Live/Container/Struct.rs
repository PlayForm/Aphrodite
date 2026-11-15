//! The envelope written beside a live container's chunks.

use serde::{Deserialize, Serialize};

/// Parsed live-container metadata envelope (the `{hash}:live:meta` inline
/// store entry written by `Hook::transform_live_container`). Unknown
/// envelope fields (e.g. the stored content `type`) are ignored on parse.
///
/// Every field defaults, because the envelope is written by a hook and read
/// here: a missing field must degrade to "not a container" rather than fail
/// the parse, and `live == false` (the default) is exactly that verdict.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Struct {
	/// `true` only for a container envelope; the default keeps a malformed
	/// entry on the plain path.
	#[serde(default)]
	#[serde(rename = "live")]
	pub(crate) Live:bool,
	/// Total bytes the container holds.
	#[serde(default)]
	#[serde(rename = "size")]
	pub(crate) Size:usize,
	/// Number of stored chunks.
	#[serde(default)]
	#[serde(rename = "chunks")]
	pub(crate) Chunks:usize,
	/// Target byte size of one chunk.
	#[serde(default)]
	#[serde(rename = "chunk_size")]
	pub(crate) ChunkSize:usize,
	/// Bytes already served; advanced and written back on every retrieve.
	#[serde(default)]
	#[serde(rename = "bytes_arrived")]
	pub(crate) BytesArrived:usize,
}
