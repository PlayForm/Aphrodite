//! The hook's answer.

use super::Struct::Struct;

/// The transformed content, or `None` when no script touched the payload.
///
/// `None` and `Some(unchanged)` are deliberately different answers: the caller uses
/// `None` to mean "no script touched this", so a script that returns its input verbatim
/// still counts as a modification. Collapsing the two would let a chain of no-op
/// scripts look like no scripting at all.
pub(crate) fn Fn(Chain:Struct) -> Option<String> { Chain.Touched.then_some(Chain.Content) }
