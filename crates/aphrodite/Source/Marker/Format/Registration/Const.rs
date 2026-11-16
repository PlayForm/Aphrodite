//! The one registration.

use std::sync::RwLock;

use super::{super::Grammar::Enum::Enum as Grammar, Struct::Struct};

/// The process-global marker registration, read on every marker render.
///
/// Process-global because the marker builder runs on paths that have no
/// `AphroditeState` (hooks, dylib, poll worker) - the same reason
/// `Preview::State::Set` is global, and the same hazard: a hot reload that
/// removes `[templates]` must publish `Grammar::Canonical` again, or a stale
/// template stays in force for the life of the process. `Registration::Set`
/// writes the whole value, so there is no way to publish half of one.
///
/// `const`-initialised rather than a `OnceLock`: the shipped configuration is a
/// real value (the canonical grammar and no hint), not an absence that has to be
/// initialised on first use.
pub(crate) static Registration:RwLock<Struct> = RwLock::new(Struct { Grammar:Grammar::Canonical, Hint:None });
