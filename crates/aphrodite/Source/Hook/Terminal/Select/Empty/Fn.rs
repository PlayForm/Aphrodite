//! The first gate: no content at all.

use crate::Hook::{Reason::Enum::Enum as Reason, Terminal::Plan::Enum::Enum as Plan};

/// Pass empty output through, before anything else is considered.
///
/// First because there is nothing to observe, nothing to split and nothing to
/// store: every later gate would be answering a question about content that does
/// not exist.
pub(crate) fn Fn(Content:&str) -> Option<Plan<'_>> { Content.is_empty().then_some(Plan::Passthrough(Reason::Empty)) }
