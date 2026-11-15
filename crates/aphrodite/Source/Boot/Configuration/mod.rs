//! Startup configuration assembly: where the config lives, what it says, and the
//! process-global registrations it drives.
//!
//! The order inside this module is load-bearing. `Path` picks the file, `Load`
//! parses it and registers the preview cap, marker templates and template tables
//! process-globally, and only then does `Boot::Run` install the tracing
//! subscriber (`crate::Logging`) - because `MultiConfiguration::Resolve` emits
//! `tracing::info!`/`warn!` diagnostics (mode fallback, port overrides, timeout
//! clamping) that would otherwise fire against no registered subscriber and be
//! silently dropped.
//!
//! `Load` produces one `Source` - the file or the command line, never both and
//! never neither - and every later step reads that value instead of re-deciding
//! which configuration is in force. `Proxies` is the only step that can fail on
//! operator error: a `[[proxies]]` entry that cannot resolve, or a standalone
//! proxy with no API key.

pub mod Compression;
pub mod Credential;
pub mod Load;
pub mod Path;
pub mod Proxies;
pub mod Source;
