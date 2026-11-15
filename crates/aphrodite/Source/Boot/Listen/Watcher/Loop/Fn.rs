//! The debounce/reload controller, exercisable without a filesystem watcher.

use std::sync::Arc;

use aphrodite::Proxy::State::Struct::Struct as AppState;

use super::super::{Apply, Debounce, Source};

/// One debounced apply per burst, for as long as the source keeps sending.
///
/// The receive is matched on the channel's own state, not on a boolean derived
/// from it. `Rx.recv().await.is_some()` throws away the one distinction the loop
/// needs: a closed channel yields `None` *immediately*, so a boolean that maps
/// closure to `false` re-enters the loop with no suspension point and burns a
/// core. Matching `None` to `break` makes a closed channel end the controller.
///
/// Nothing spins today - `Tx` is moved into the `notify` closure held by the
/// watcher this source owns, so no sender can be dropped while the loop runs -
/// but that is a property of the sender's scope, not of this loop. Shortening
/// the sender's life is exactly the refactor the old arm invited, so the shape
/// is fixed here rather than left as a trap.
pub(crate) async fn Fn(Source:Source::Struct::Struct, States:Vec<Arc<AppState>>, Path:String) {
	let mut Rx = Source.Rx;

	// The channel's own state is the loop condition: a closed channel ends the
	// controller instead of re-entering with no suspension point.
	while let Some(()) = Rx.recv().await {
		Debounce::Fn::Fn(&mut Rx).await;
		Apply::Fn::Fn(&States, &Path);
	}
}
