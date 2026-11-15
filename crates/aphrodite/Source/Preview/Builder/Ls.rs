//! Directory-listing preview arm.
//!
//! Parse each entry, aggregate the file/directory counts and the extension tally, then
//! render that tally. It is not a filesystem query - nothing is stat'ed, so a listing of
//! files that no longer exist previews the same way.

use crate::Preview::{Input::Struct::Struct as Input, Parse, Render};

/// Build a preview of a directory listing.
///
/// Promises file and directory counts plus the most common extensions. It is not a tree:
/// nesting is flattened, so a listing of one directory of directories and a deep tree both
/// report a file count and nothing about depth.
pub(crate) fn Fn(Inp:&Input<'_>) -> String { Render::Listing::Fn::Fn(&Parse::Listing::Fn::Fn(Inp), Inp.Total) }
