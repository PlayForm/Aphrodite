//! The envelope written beside a live container's chunks.

use serde::{Deserialize, Serialize};

/// Parsed live-container metadata envelope (the `{hash}:live:meta` inline
/// store entry written by `Hook::transform_live_container`). Unknown
/// envelope fields (e.g. the stored content `type`) are ignored on parse.
///
/// Every field defaults, because the envelope is written by a hook and read
/// here: a missing field must degrade to "not a container" rather than fail
/// the parse, and `Enable == false` (the default) is exactly that verdict.
///
/// The names are the envelope's own `snake_case` JSON keys, pinned per field.
/// The envelope is a contract between the hook that writes it and this read, so
/// the identifiers moved and the keys did not.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Struct {
	/// `live`; `true` only for a container envelope. The default keeps a
	/// malformed entry on the plain path, which is why the flag is not the
	/// presence of the entry itself.
	#[serde(default)]
	#[serde(rename = "live")]
	pub(crate) Enable:bool,
	/// `size`; total bytes the container holds. `0` is a container with no
	/// content, never "unknown".
	#[serde(default)]
	#[serde(rename = "size")]
	pub(crate) Size:usize,
	/// `chunks`; number of stored chunks. `0` is a degenerate envelope - a
	/// container claiming nothing to reassemble - and the read refuses it.
	#[serde(default)]
	#[serde(rename = "chunks")]
	pub(crate) Count:usize,
	/// `chunk_size`; target byte size of one chunk. `0` means the writer chose
	/// no target, and no reader consults it for reassembly.
	#[serde(default)]
	#[serde(rename = "chunk_size")]
	pub(crate) Granularity:usize,
	/// `bytes_arrived`; bytes already served, advanced and written back on
	/// every retrieve. `0` means the container has not been read yet.
	#[serde(default)]
	#[serde(rename = "bytes_arrived")]
	pub(crate) Arrived:usize,
}
