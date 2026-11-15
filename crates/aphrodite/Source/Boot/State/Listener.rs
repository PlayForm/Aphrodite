//! The bound-but-not-yet-serving listener record.

use std::sync::Arc;

use aphrodite::{Configuration::CLI::Struct::Struct as CLI, Proxy::State::Struct::Struct as AppState};
use tokio::net::TcpListener;

/// One proxy listener: configured, bound and stateful, but not yet serving.
///
/// Carrying all four pieces together is what makes the bind-before-serve
/// ordering enforceable: a `Bound` can only exist after its port was taken and
/// its `AppState` built, so a failure in either step aborts startup instead of
/// leaving a listener silently dead while its sibling keeps answering.
pub struct Bound {
	/// The listener's configured name, or its listen address when unnamed.
	pub Name:String,
	/// The resolved per-listener configuration.
	pub CLI:CLI,
	/// The socket, already bound so a conflict surfaces before anything serves.
	pub Listener:TcpListener,
	/// The state this listener's routes and the config watcher share.
	pub State:Arc<AppState>,
}
