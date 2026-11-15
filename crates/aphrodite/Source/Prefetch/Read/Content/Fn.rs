//! Read one path's content, refusing anything over the size cap.

use std::path::Path;

use crate::Prefetch::{Outcome::Enum::ReadOutcome, Read::Const::MaximumPrefetchSize};

/// The size check, then the read.
///
/// `None` means "no outcome yet" - the caller turns it into an error - so a
/// missing `metadata` and a failed `read_to_string` need no separate arms. The
/// size is carried into `Loaded` because the caller reports it and a second
/// `metadata` call could disagree with the first.
///
/// The read is a fallback: a file over the cap is never opened, which is the whole
/// point of the cap - prefetching a huge file would evict everything already
/// stored for content nobody asked to see yet.
pub(crate) fn Fn(path:&Path) -> Option<ReadOutcome> {
	let Size = std::fs::metadata(path).ok()?.len();
	let Over = (Size > MaximumPrefetchSize).then_some(ReadOutcome::SkippedSize { Size });

	Over.or_else(|| {
		std::fs::read_to_string(path)
			.ok()
			.map(|Content| ReadOutcome::Loaded { Content, Size })
	})
}
