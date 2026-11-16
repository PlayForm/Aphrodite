//! Reading the registration.

use super::super::{Const::Registration, Struct::Struct};

/// Run `Read` against the registration currently in force.
///
/// A closure rather than a getter because the renderer needs both fields for the
/// length of one render and nothing else: returning them would clone the format
/// string and the hint on every marker, and returning a guard would leak the
/// lock discipline into every consumer. The guard never leaves this function, so
/// "the registration is held for one render" is a property of the type rather
/// than a rule each caller re-earns.
///
/// A poisoned lock is recovered rather than propagated: one panicking render
/// must not stop every later marker in the process from being built.
pub(crate) fn Fn<T>(Read:impl FnOnce(&Struct) -> T) -> T {
	Read(&Registration.read().unwrap_or_else(std::sync::PoisonError::into_inner))
}
