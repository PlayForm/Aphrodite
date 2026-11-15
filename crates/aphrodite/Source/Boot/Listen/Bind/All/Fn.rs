//! Binding every listener, in order, before any of them serves.

use aphrodite::Configuration::{
	CLI::Struct::Struct as CLI,
	Proxy::Compression::Struct::Struct as CompressionConfiguration,
};
use futures::{StreamExt, TryStreamExt};

use super::super::{super::super::State::Listener::Bound, One};

/// Bind every listener and build its `AppState`, returning them ready to serve.
///
/// Returns `Err` on the first bind or state failure, which aborts startup
/// entirely. That is deliberate: a listener that failed to bind but was skipped
/// would leave the process running and answering on its siblings while the
/// operator believed both were up. Before this existed, each listener bound its
/// own port deep inside its spawned task, so a port conflict logged an error and
/// left that one listener silently dead while the other kept serving - the
/// process exited 0 and looked healthy.
///
/// `then` is sequential on purpose. Binding concurrently would make which
/// listener loses a port conflict depend on scheduling, and the whole point of
/// binding here is that the failure names the entry the operator has to fix.
pub(crate) async fn Fn(
	Proxies:Vec<(String, CLI)>,
	Compression:Option<&CompressionConfiguration>,
) -> anyhow::Result<Vec<Bound>> {
	futures::stream::iter(Proxies)
		.then(|(Name, CLI)| One::Fn::Fn(Name, CLI, Compression))
		.try_collect()
		.await
}
