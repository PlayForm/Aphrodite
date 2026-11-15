//! `Reject`: a predicate turned into a lazily-built error.

/// Reject when `Reject` holds, building `Error` only in that case.
///
/// `then` and not `then_some`: `then_some(Error())` would construct the error
/// on the accepted path too, which is the whole cost this primitive exists to
/// avoid. `map_or` turns the absence of a rejection into `Ok(())`.
///
/// The boolean decision survives exactly once - here. A validator composes
/// three of these with `?` and never mentions the predicate again.
pub(crate) fn Fn<E>(Reject:bool, Error:impl FnOnce() -> E) -> Result<(), E> { Reject.then(Error).map_or(Ok(()), Err) }
