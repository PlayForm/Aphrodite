//! Startup configuration assembly: where the config lives, what it says, and the
//! process-global registrations it drives.
//!
//! The order inside this module is load-bearing. `Resolve` picks the file,
//! `Load` parses it and registers the preview cap, marker templates and template
//! tables process-globally, and only then does `Boot::Run` install the tracing
//! subscriber (`crate::Logging`) - because `MultiConfig::resolve()` emits
//! `tracing::info!`/`warn!` diagnostics (mode fallback, port overrides,
//! timeout clamping) that would otherwise fire against no registered subscriber
//! and be silently dropped.
//!
//! `Assemble` is the last step and the only one that can fail on operator error:
//! a standalone proxy with no API key.

pub mod Compression;
pub mod Load;
pub mod Path;
pub mod Proxies;
