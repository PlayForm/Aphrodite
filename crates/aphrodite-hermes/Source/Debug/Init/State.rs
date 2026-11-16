//! The recorded ownership decision, as a process-global.

use std::sync::OnceLock;

use crate::Debug::Init::Ownership;

/// The one install decision this process ever makes.
///
/// `OnceLock::get_or_init` is both the "once" and the record: the first caller
/// runs `try_init` and stores what happened, every later caller reads the stored
/// answer with one atomic load. `Once` plus a separate flag would be the same
/// thing spelled twice, and the two could disagree.
pub(crate) static INSTALL:OnceLock<Ownership::Enum::Enum> = OnceLock::new();
