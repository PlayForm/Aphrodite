//! One prepared context-engine archive.

/// Everything the archive needs, built before any state is touched.
///
/// `Packed` is the JSON payload the marker replaces; it is carried rather than
/// rebuilt because its length is the marker's `size`, its bytes are the store
/// key's input, and its first 120 characters are the preview. `Prompt` is the
/// marker plus the rendered offload prompt, ready for the caller to inject.
pub struct Struct {
	/// Turns protected at the start of the index.
	pub HeadN:usize,
	/// Turns protected at the end of the index.
	pub TailN:usize,
	/// Turns this archive replaces.
	pub MiddleN:usize,
	/// The index keys being replaced, oldest first.
	pub MiddleKeys:Vec<usize>,
	/// The packed payload, exactly as it will be stored.
	pub Packed:String,
	/// Content hash of `Packed` - the store key and the marker's hash.
	pub Hash:String,
	/// The preview the model will read for the archive.
	pub Preview:String,
	/// The marker plus the rendered offload prompt.
	pub Prompt:String,
}
