//! The running remainder of one preview's budget.

use std::ops::ControlFlow;

/// How much signature text one preview may still carry.
///
/// A shared counter with one operation, so the policy that stops a scan exists once instead of as a
/// sign test repeated before every line. The `+1` in the charge is the separator the renderer will
/// insert between two signatures, so it must stay in the charge and not move to the render - moving it
/// would let a preview overrun its total by one byte per line.
///
/// It is not a per-category cap: the same value is spent by every category of one extraction, in the
/// order `Policy` gives them.
pub(crate) struct Struct {
	Remaining:isize,
}

impl Struct {
	pub(crate) fn new(Maximum:usize) -> Self { Self { Remaining:Maximum as isize } }

	/// Charge one admitted signature and say whether the scan may continue.
	///
	/// `ControlFlow` rather than a unit-error sentinel: exhaustion is completion, not failure, and the
	/// previous sentinel made the type claim a walk had failed when it had simply finished. The charge
	/// happens before the test, so the signature that spends the budget is the last one kept - which is
	/// what the previous decrement-then-test did.
	pub(crate) fn Charge(&mut self, Length:usize) -> ControlFlow<(), ()> {
		self.Remaining -= Length as isize + 1;

		// Exhaustion is the value the caller matches: `Some` when the charge spent the remainder.
		self.Exhausted()
			.then_some(())
			.map_or(ControlFlow::Continue(()), |_| ControlFlow::Break(()))
	}

	fn Exhausted(&self) -> bool { self.Remaining <= 0 }
}
