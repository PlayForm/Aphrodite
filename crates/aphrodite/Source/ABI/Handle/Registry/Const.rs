//! The one table.

use super::Struct::Struct;

/// Process-global handle table.
///
/// A `static`, not a `OnceLock`: the registry is a plain value with `const`
/// mutexes inside, so there is nothing to initialise lazily at the registry
/// level - the map inside it is the only lazy part, and `With` owns that.
pub(crate) static Handles:Struct = Struct::New();
