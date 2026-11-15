//! Building one listener's `AppState`.

use std::sync::Arc;

use aphrodite::{
	Configuration::{CLI::Struct::Struct as CLI, Proxy::Compression::Struct::Struct as CompressionConfiguration},
	Proxy,
	Proxy::State::Struct::Struct as AppState,
};

/// Build the shared `AppState` for one listener.
///
/// Returns an `Arc` because three owners hold it for the process lifetime: the
/// serving task, the router's state extractor, and the config-file watcher.
/// Fails when the CCR store cannot be opened - a listener without its store would
/// answer every request while silently dropping every marker, so this must abort
/// startup rather than degrade.
pub async fn Build(CLI:&CLI, Compression:Option<&CompressionConfiguration>) -> anyhow::Result<Arc<AppState>> {
	Ok(Arc::new(Proxy::State::Build::Fn::Fn(CLI, Compression).await?))
}
