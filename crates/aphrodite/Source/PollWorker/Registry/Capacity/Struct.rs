//! The registry's capacity, as a type that cannot be zero.

use std::num::NonZeroUsize;

use crate::PollWorker::Task::Const::Maximum;

/// How many tasks the registry may hold.
///
/// A zero capacity is not a valid state - a registry that can hold nothing would
/// evict on every insert and never track a task - so the only constructor is the
/// policy constant itself, checked at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Capacity(NonZeroUsize);

impl Capacity {
	/// The shipped policy bound (`Maximum`), as a non-zero value.
	pub(crate) const Policy:Capacity = match NonZeroUsize::new(Maximum) {
		Some(Value) => Capacity(Value),
		None => panic!("Maximum must be non-zero"),
	};

	/// The bound as a plain count.
	pub(crate) const fn Get(&self) -> usize { self.0.get() }
}
