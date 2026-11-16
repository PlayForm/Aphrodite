//! The debounce/reload controller, exercisable without a filesystem watcher.

use std::sync::Arc;

use aphrodite::Proxy::State::Struct::Struct as AppState;

use super::super::{Apply, Debounce, Source};

/// One settled apply per burst, for as long as the source keeps sending.
///
/// The loop condition is the channel's own state: a closed channel yields `None`
/// immediately, so a boolean derived from it re-enters with no suspension point and
/// burns a core. Matching `None` to `return` ends the controller instead.
///
/// A burst that ends because the channel closed is still applied before the loop
/// ends: the event that started it was real, and the pending change is the last thing
/// this controller can honour. Only then does the loop stop.
///
/// The path is no longer a parameter. The event carries the resolved target, so there
/// is one copy of "which file changed" and nothing to keep in step with the channel.
pub(crate) async fn Fn(Source:Source::Struct::Struct, States:Vec<Arc<AppState>>) {
	let mut Rx = Source.Rx;

	while let Some(Target) = Rx.recv().await {
		let Outcome = Debounce::Fn::Fn(&mut Rx).await;

		Apply::Fn::Fn(&States, &Target);

		if let Debounce::Outcome::Enum::Enum::Closed = Outcome {
			return;
		}
	}
}
