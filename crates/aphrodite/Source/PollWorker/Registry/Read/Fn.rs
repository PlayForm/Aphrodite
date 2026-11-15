//! Open the read-only view over a registry.

use crate::PollWorker::Registry::{ReadFrom::Fn::Fn as ReadFrom, Reader::Struct::Reader, Struct::Registry};

/// The read-only view of `Self`.
pub(crate) fn Fn<'a>(This:&'a Registry<'_>) -> Reader<'a> { ReadFrom(This.Tasks) }
