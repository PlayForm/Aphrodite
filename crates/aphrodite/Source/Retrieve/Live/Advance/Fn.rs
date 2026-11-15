//! Advance a live container's progress.

use crate::Retrieve::{
	Live::Container::Struct::Struct as LiveContainer,
	Page::Mode::Enum::Enum as Mode,
	Progress::Struct::Struct as Progress,
};

/// The progress a container retrieve reports, after this response.
///
/// The container's own size is what "delivered" means when the request asks for the
/// document to its end - an `Exact` retrieve, or a window with an unlimited tail - because
/// that response carries the remainder. Any other window carries part of it, so the bytes
/// this response served are added to what arrived before, capped at the container's size.
///
/// `Complete` is derived from `BytesArrived` rather than carried alongside it: the old pair
/// `Complete = FinalResolve || BytesArrived >= Size` could disagree with its own arithmetic,
/// and with the size substituted for a final resolve the two always agree, so the second
/// flag had nothing left to say.
///
/// A partial window's count includes the `[lines ...]` header, as it always has: the field
/// is an approximation of what the caller received, not a byte count of the document.
pub(crate) fn Fn(Mode:&Mode, Container:&LiveContainer, Served:usize) -> Progress {
	let Served = match Mode {
		Mode::Exact | Mode::Window { Limit: None, .. } => Container.Size,
		Mode::Window { .. } => Served.min(Container.Size),
	};
	let BytesArrived = Container.BytesArrived.saturating_add(Served).min(Container.Size);

	Progress {
		Chunks:Container.Chunks,
		ChunkSize:Container.ChunkSize,
		BytesTotal:Container.Size,
		BytesArrived,
		Complete:BytesArrived >= Container.Size,
	}
}
