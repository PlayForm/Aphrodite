//! Serving one bound listener until shutdown.

use std::net::SocketAddr;

use aphrodite::Configuration::Proxy::Mode::Enum::Enum as ProxyMode;

use crate::{
	Boot::{Route, State::Listener::Bound},
	Guard::Token::Fn::MgmtToken,
};

/// The one-time notice printed when the management routes carry no credential.
///
/// Kept as data so the message is one string in one place, and so the gate that
/// decides to print it stays a value rather than a branch.
const MGMT_NOTICE:&str = "APHRODITE_MGMT_TOKEN not set - management routes (/stats, /retrieve, /ccr/*, /reload, \
                          /tool/relay, ...) accept any loopback caller with no credential";

/// Serve one bound listener until its shutdown receiver fires.
///
/// The router is built here, per listener, because each listener has its own
/// `AppState` - merging two listeners' routes into one router would share one
/// state across both and send cache traffic to the token proxy.
///
/// On shutdown, background tasks are drained *before* a serve error is
/// propagated: a leaked tool-relay callback outlives the response that scheduled
/// it, and reporting the serve error first would exit with those tasks still
/// holding the store open.
pub async fn Serve(Bound:Bound, mut ShutdownRx:tokio::sync::watch::Receiver<bool>) -> anyhow::Result<()> {
	let Bound { Name, CLI, Listener, State } = Bound;
	let TaskTracker = State.TaskTracker.clone();

	let ModeStr = match CLI.Mode {
		ProxyMode::Cache => "cache",
		ProxyMode::Token => "token",
	};

	tracing::info!(
		name = %Name,
		Listen = %CLI.Listen,
		Mode = %ModeStr,
		APIURL = %CLI.APIURL,
		Model = %CLI.Model,
		ToolRelay = CLI.ToolRelay,
		"proxy starting"
	);

	// 02-F1: one-time notice, not per-request - management routes
	// (/stats, /retrieve, /ccr/*, /reload, /tool/relay, ...) accept any
	// loopback caller with no credential until APHRODITE_MGMT_TOKEN is set.
	MgmtToken()
		.is_none()
		.then_some(MGMT_NOTICE)
		.into_iter()
		.for_each(|Notice| tracing::warn!(name = %Name, "{}", Notice));

	let App = Route::Build::Build(State);

	// F9: `Listener` was already bound in `run()` before any proxy task was
	// spawned - see the call site for why.
	tracing::info!(Addr = %Listener.local_addr()?, "listening");

	// Graceful shutdown triggered by shared watch channel - fires when main()
	// calls shutdown_tx.send(true) after OS signal, or when sender is dropped.
	let ShutdownFut = async move {
		let _ = ShutdownRx.changed().await;
	};
	let ServeResult = axum::serve(Listener, App.into_make_service_with_connect_info::<SocketAddr>())
		.with_graceful_shutdown(ShutdownFut)
		.await;

	// Close task tracker BEFORE propagating serve error so background tasks
	// are not leaked when serve fails (e.g. epoll registration error).
	TaskTracker.close();
	TaskTracker.wait().await;
	tracing::debug!("all background tasks completed");

	ServeResult?;

	Ok(())
}
