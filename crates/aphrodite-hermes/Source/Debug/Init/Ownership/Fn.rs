//! Reading the recorded ownership decision without making it.

use crate::Debug::Init::{Ownership, State::Install};

/// The ownership decision this process recorded, or `None` if no install has
/// been attempted yet.
///
/// Read-only on purpose: asking "who owns logging?" must never be the thing
/// that claims it. `Debug::Init::Fn::Fn` is the only writer, and it is called
/// from the entry points that genuinely start a turn (`Hook::Call`,
/// `Tool::Dispatch`, `Directive::Into`), so a warning that fires before any of
/// them still gets the honest answer - "nobody has decided yet" - and takes the
/// safe branch instead of quietly installing a subscriber as a side effect of
/// warning about something.
pub(crate) fn Fn() -> Option<&'static Ownership::Enum::Enum> { Install.get() }
