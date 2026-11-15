//! The public Stage-2 entry point.

use crate::Stage2::{Compress::Decide::Fn::Fn as Decide, Outcome::Option::Fn::Fn as AsOption};

/// Produce a semantically reduced version of content.
///
/// Returns `Some(reduced_string)` when reduction was beneficial, or `None` when the content is
/// too small, no reducer exists for the type, the reducer did not apply, or it produced no
/// savings. The four reasons are distinguished internally by `Outcome`; this signature is the
/// public contract and its callers only ever need the replacement-or-not answer.
pub fn CompressStage2(Content:&str, CCRType:&str) -> Option<String> { AsOption(Decide(Content, CCRType)) }
