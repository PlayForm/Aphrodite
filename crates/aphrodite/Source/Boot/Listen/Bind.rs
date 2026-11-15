//! Binding every listener and building its state, before anything serves.

use anyhow::Context;
use aphrodite::Configuration::{
	CLI::Struct::Struct as CLI,
	Proxy::Compression::Struct::Struct as CompressionConfiguration,
};
use futures::{StreamExt, TryStreamExt};

use crate::Boot::{State, State::Listener::Bound};

/// Bind every listener and build its `AppState`, returning them ready to serve.
///
/// Returns `Err` on the first bind or state failure, which aborts startup
/// entirely. That is deliberate: a listener that failed to bind but was skipped
/// would leave the process running and answering on its siblings while the
/// operator believed both were up.
pub async fn Bind(
	Proxies:Vec<(String, CLI)>,
	Compression:Option<&CompressionConfiguration>,
) -> anyhow::Result<Vec<Bound>> {
	// F9: bind every listener BEFORE spawning any proxy task. Previously
	// each `run_single` bound its own port deep inside its spawned task, so
	// a bind failure (port already in use - a stale instance, a conflict
	// between the cache/token pair) only logged an error and left that one
	// listener silently dead while the other kept serving - the process
	// exited 0 and looked healthy. Binding here means a failure aborts
	// startup entirely, loudly, before any listener goes live.
	//
	// `AppState` is also built here now, not inside `run_single` (report 07
	// F4/T15) - the config-file watcher below needs every listener's
	// `Arc<AppState>` up front so a file change can actually apply new
	// compression thresholds to the live proxies, not just re-parse and log.
	let Bound:Vec<Bound> = futures::stream::iter(Proxies)
		.then(|(name, mut CLI)| {
			async move {
				// Resolve relative ccr_db_path against the binary directory, not CWD.
				// This way the database path is stable regardless of where the
				// process is launched from. (Moved here from `run_single` so
				// `build_state` can run before spawning.)
				match CLI.CCRDatabasePath.as_ref() {
					Some(DatabasePath) => {
						match !DatabasePath.as_os_str().is_empty() && !DatabasePath.is_absolute() {
							true => {
								match std::env::current_exe()
									.ok()
									.and_then(|ExePath| ExePath.parent().map(|Parent| Parent.to_path_buf()))
								{
									Some(ExeDirectory) => {
										let Old = DatabasePath.display().to_string();
										CLI.CCRDatabasePath = Some(ExeDirectory.join(DatabasePath));
										tracing::info!(
											"resolved relative ccr_db_path from {} to {}",
											Old,
											CLI.CCRDatabasePath.as_ref().unwrap().display()
										);
									},
									None => {},
								}
							},
							false => {},
						}
						match CLI.CCRDatabasePath.as_ref().and_then(|p| p.parent()) {
							Some(Parent) => {
								std::fs::create_dir_all(Parent)?;
							},
							None => {},
						}
					},
					None => {},
				}
				let Listener = tokio::net::TcpListener::bind(CLI.Listen)
					.await
					.with_context(|| format!("failed to bind listener \"{name}\" on {}", CLI.Listen))?;
				let State = State::Build::Build(&CLI, Compression).await?;
				Ok::<Bound, anyhow::Error>(Bound { name, CLI, Listener, State })
			}
		})
		.try_collect()
		.await?;
	Ok(Bound)
}
