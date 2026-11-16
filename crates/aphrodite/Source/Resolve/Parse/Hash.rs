//! Hash extraction from one marker.
//!
//! A marker body is `hash|type|size`, and only the first field is the key the store
//! answers to. This module owns that split, and it BORROWS: the hash is a slice of the
//! marker, which is a slice of the document, so extracting it allocates nothing and cannot
//! produce a hash that is not literally in the text.
//!
//! It is not a validator - an unknown hash parses and then misses the store, which is the
//! resolver's job to report and the caller's argument bug to fix at its own boundary
//! (`Marker::Parse::Normalize`). But an EMPTY hash field is not a hash at all:
//! `<<<CCR:>>>` and `<<<CCR:|text|1>>>` name nothing, so they are not references and the
//! scanner does not emit them (see [`super::Tolerance`]).

use super::{Prefix, Suffix};

/// The hash field of a complete marker, or `None` when there is nothing to look up.
///
/// `None` covers two cases the caller treats identically: the text is not a complete
/// marker (a delimiter is missing or out of order), or the body's first field is empty.
/// Both mean "this is not a reference".
///
/// It does not trim: a hash with stray whitespace parses as-is and then misses the store,
/// which is the caller's argument bug to fix at its own boundary.
pub(crate) fn Fn(Marker:&str) -> Option<&str> {
	let Inner = Marker.strip_prefix(Prefix::Const)?.strip_suffix(Suffix::Const)?;

	Inner.split('|').next().filter(|Hash| !Hash.is_empty())
}
