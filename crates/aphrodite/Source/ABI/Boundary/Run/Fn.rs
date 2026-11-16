//! `Run`: an operation under `catch_unwind`, rendered either way.

use std::os::raw::c_char;

use crate::ABI::Boundary::Panic::Fn::Fn as Dispose;

/// Run `Operation` and render its outcome, converting a panic into an error
/// payload instead of unwinding across the `extern "C"` boundary (which triggers
/// the Rust runtime's forced process abort).
///
/// `AssertUnwindSafe` is asserted here, once, and the assertion is the ABI's
/// contract rather than a convenience: a panic is converted into an error string
/// that the caller reads, no invariant crosses the boundary in either direction,
/// and a session whose state was half-mutated is documented as observably degraded
/// (`Handle::Lookup::Guarded`'s torn-state contract) rather than assumed clean.
///
/// The caught payload does not end here as a value to drop: it is handed to
/// `Boundary::Panic`, which owns the disposition policy. That matters because
/// dropping it in this frame would run a destructor the panicking code chose,
/// outside the catch that produced it - see that leaf.
///
/// The message is `"internal error: panicked"` and it is a contract string: the
/// exports that do not take a handle have always answered it, while the ones that
/// go through `Guarded` answer `"internal error: hook panicked"` from inside that
/// wrapper, which catches before this frame sees the unwind.
///
/// Rule: every new `pub unsafe extern "C" fn` added to this crate routes through
/// here (directly, or via `Boundary::State`) - there is no second shape that stays
/// panic-safe across the FFI boundary.
pub(crate) fn Fn(Operation:impl FnOnce() -> anyhow::Result<crate::ABI::Boundary::Answer::Enum::Enum>) -> *mut c_char {
	crate::ABI::Boundary::Render::Fn::Fn(
		std::panic::catch_unwind(std::panic::AssertUnwindSafe(Operation))
			.map_err(|Payload| Dispose(Payload, "internal error: panicked"))
			.and_then(|Inner| Inner),
	)
}
