//! One path's outcome, as the response reports it.

use crate::State::Identity::Content::Struct::ContentHash;

/// What one requested path produced, in the shape the report needs.
///
/// The four cases are the four answers a caller can act on, and each carries exactly
/// what its sentence names: a loaded file's identity and size, a refused path's reason,
/// an oversize file's real size and the cap it broke, a missing path's name. The report
/// derives its counts by counting these, so the totals and the list cannot disagree.
#[derive(Debug, Clone)]
pub(crate) enum Record {
	/// Stored: the content is in the inline store under `Hash`.
	Loaded {
		/// The path the content came from.
		Path:String,
		/// Content identity, resolvable through the store.
		Hash:ContentHash,
		/// Detected content type.
		Type:String,
		/// Length of the stored payload.
		Size:usize,
		/// The preview the marker carries.
		Preview:String,
	},
	/// Readable but over the per-file cap, so never opened for storage.
	Skipped {
		/// The path that was skipped.
		Path:String,
		/// The size the filesystem reported.
		Size:u64,
		/// The cap it exceeded.
		Maximum:u64,
	},
	/// Refused by the prefetch guard - never read, never stored.
	Refused {
		/// The path that was refused.
		Path:String,
		/// Why, in the guard's own words.
		Reason:String,
	},
	/// Absent or unreadable: nothing was read and nothing will be stored.
	Missing {
		/// The path that produced nothing.
		Path:String,
	},
}
