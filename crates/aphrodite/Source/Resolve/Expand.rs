//! Resolution entry point: a hash in, the content it names out.
//!
//! Every retrieval path in the crate funnels here - the ABI retrieve export, the proxy relay,
//! the hooks, the Hermes retrieve tool - so this function owns the whole nesting contract:
//! resolve the hash, resolve the markers inside it, resolve theirs, up to
//! `AphroditeState::RecursiveDepth`. It is not a store lookup; `One.rs` is the lookup, and
//! this wraps it.
//!
//! It is also the retrieval metric's only sampling point: the timing, the found/miss counter
//! and the one structured line per request are recorded here, so a caller that bypasses this
//! function is invisible to the metrics - which is why the ABI retrieve path, which resolves
//! directly, has to attribute its own consequence signals.
//!
//! This is the UNBOUNDED contract, and the only place the two-type expansion split is
//! projected back to text: a caller of `Expand` wants the document, and the traversal it builds
//! here has no budget, so it can only ever stop because the hash is absent.

use crate::{
	Resolve::{Expansion, Recursive::Recursive, Traversal::Struct::Struct as Traversal},
	State::Struct::AphroditeState,
};

/// Expand a hash through every nesting level.
///
/// Returns the fully expanded content, or `None` when the top-level hash is not in the store.
/// A nested hash that cannot be resolved is left as its original marker text rather than
/// replaced with a placeholder - that entry may only be evicted or late, and a placeholder
/// baked into the output can never heal.
///
/// It is not a compressor and it never writes back: the stored bytes under a hash must stay
/// byte-identical to what the hash names, so an expanded result is returned and forgotten.
/// Writing it back breaks the content-address invariant and destroys the pristine original
/// irrecoverably - including any marker-shaped text the original merely contained.
pub fn Fn(State:&mut AphroditeState, HashValue:&str) -> Option<String> {
	// DEV-ONLY debug instrumentation (crate::Debug): retrieval metrics + one structured line
	// per retrieval request (hash, found, depth cap, elapsed). Captured only when the logger
	// is on.
	let T0 = crate::Debug::Level::Enabled::Fn::Fn().then(std::time::Instant::now);
	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::Retrieves);

	// The traversal borrows the state for exactly as long as the walk runs, so the metrics
	// below can read it again - and nothing the walk does can touch anything but the store
	// and the depth cap.
	let Out = {
		let mut Traversal = Traversal::New(State);

		Recursive::Fn(&mut Traversal, HashValue, 0)
	}
	.ok()
	.map(Expansion::Enum::Enum::IntoText);

	// `inspect` runs only when the timing gate was taken, which is the whole reason the gate
	// is an `Option` rather than a boolean: a disabled logger must not pay for a clock read, a
	// counter or a formatted line.
	let _ = T0.inspect(|T0| {
		let Us = T0.elapsed().as_micros() as u64;
		crate::Debug::Timing::Add::Fn::Fn(crate::Debug::Timing::Enum::Enum::Retrieve, Us);
		crate::Debug::Log::At::Fn::Fn(
			State.DebugLevel,
			"retrieve",
			&[
				("hash", HashValue.to_string()),
				("found", Out.is_some().to_string()),
				("depth_cap", State.RecursiveDepth.to_string()),
				("elapsed_us", Us.to_string()),
			],
		);

		// A miss degrades to a warning (dev metrics).
		let _ = Out.is_none().then(|| {
			crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::RetrieveMisses);
			crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::Warnings);
		});
	});

	Out
}
