//! The only constructor of an accepted reduction.

use crate::Stage2::Reduction::Struct::Struct as Reduction;

/// Admit a candidate only when it is strictly shorter than the original.
///
/// The comparison is on bytes, which is the unit the context budget is spent in - a candidate
/// that is shorter in characters but longer in bytes is not a saving. It is not a quality check:
/// a shorter candidate that loses information is a defect in the reducer, and the reducers are
/// what decide what to keep.
pub(crate) fn Fn(Candidate:String, OriginalLength:usize) -> Option<Reduction> {
	(Candidate.len() < OriginalLength).then_some(Reduction { Content:Candidate })
}
