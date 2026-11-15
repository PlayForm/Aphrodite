//! Binding one listener and building its state.

use anyhow::Context;
use aphrodite::Configuration::{
	CLI::Struct::Struct as CLI,
	Proxy::Compression::Struct::Struct as CompressionConfiguration,
};

use super::super::Database;
use crate::Boot::{State, State::Listener::Bound};

/// Bind one listener and build its `AppState`, ready to serve.
///
/// The order is the contract: the database path is resolved and its directory
/// created first, the socket is bound second, and the state is built last. A
/// listener that bound but whose store cannot be opened would answer every
/// request while dropping every marker, so state construction happens before the
/// socket is handed to a serving task and its failure aborts startup.
///
/// The bind failure names both the listener and the address, because the caller
/// sees only the first error in a sequential bind and needs to know which
/// `[[proxies]]` entry lost the port.
pub(crate) async fn Fn(
	Name:String,
	mut CLI:CLI,
	Compression:Option<&CompressionConfiguration>,
) -> anyhow::Result<Bound> {
	Database::Fn::Fn(&mut CLI)?;

	let Listener = tokio::net::TcpListener::bind(CLI.Listen)
		.await
		.with_context(|| format!("failed to bind listener \"{Name}\" on {}", CLI.Listen))?;

	let State = State::Build::Build(&CLI, Compression).await?;

	Ok(Bound { Name, CLI, Listener, State })
}
