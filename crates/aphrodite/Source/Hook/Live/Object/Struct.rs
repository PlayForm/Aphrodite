//! A live container as one storage object.

/// A live container: one object, stored as several views.
///
/// The container used to be three separate insertions that had to agree about a
/// hash, a total and a chunk count. They are one object here: every key, every
/// envelope field and every payload field is derived from this value, so the
/// views cannot disagree about what they are views of. The count is not an
/// estimate either - it is what the install pass actually stored.
pub(crate) struct Struct {
	/// Content hash of the whole content: the round-trip key and the marker's
	/// hash, and the prefix every view's key is derived from.
	pub(crate) Hash:String,
	/// The content's own reported type, carried in the envelope.
	pub(crate) Type:String,
	/// Total bytes the container holds.
	pub(crate) Total:usize,
	/// Chunks actually stored for this container.
	pub(crate) Chunks:usize,
	/// Target byte size of one chunk, as the policy resolved it.
	pub(crate) ChunkSize:usize,
}

impl Struct {
	/// The metadata envelope written beside the chunks.
	///
	/// A wire contract with `Retrieve::Live::Container::Struct`: the field names
	/// and the `bytes_arrived` starting point are what the retrieve path parses
	/// and advances, so a field renamed here silently demotes every container to
	/// a plain entry.
	pub(crate) fn Meta(&self) -> String {
		serde_json::json!({
			"live": true,
			"type": self.Type,
			"size": self.Total,
			"chunks": self.Chunks,
			"chunk_size": self.ChunkSize,
			"bytes_arrived": 0,
		})
		.to_string()
	}
}
