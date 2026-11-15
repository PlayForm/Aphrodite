//! `DirectivesDirectory`: the home-namespace directive store.

use std::path::PathBuf;

use crate::Home::Runtime::Fn::Fn as RuntimeHome;

/// `directives/` under the runtime home (the home-namespace directive store).
///
/// This is the writable user store, distinct from the built-in directives
/// compiled into the crate: a user file shadows a built-in of the same name,
/// so pointing this at the wrong directory silently changes which directives
/// the model is given.
pub fn Fn() -> PathBuf { RuntimeHome().join("directives") }
