//! The state the listeners and their routes share.
//!
//! Two things live here: the `AppState` each listener builds once at startup
//! (`Build`) and the record that carries it - together with the bound socket and
//! the resolved CLI - from the bind step to the serve step (`Listener`). The
//! record exists so the config-file watcher can reach every live `Arc<AppState>`
//! and apply new compression thresholds to all of them, not just re-parse and
//! log.

pub mod Build;
pub mod Listener;

