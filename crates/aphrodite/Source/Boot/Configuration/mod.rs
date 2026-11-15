//! Startup configuration assembly: where the config lives, what it says, and the
//! process-global registrations it drives.
//!
//! The order inside this module is load-bearing. `Resolve` picks the file,
//! `Load` parses it and registers the preview cap, marker templates and template
//! tables process-globally, and only then does `Subscribe` install the tracing
//! subscriber - because `MultiConfig::resolve()` emits `tracing::info!`/`warn!`
//! diagnostics (mode fallback, port overrides, timeout clamping) that would
//! otherwise fire against no registered subscriber and be silently dropped.
//!
//! `Assemble` is the last step and the only one that can fail on operator error:
//! a standalone proxy with no API key.

mod Compression;
mod Load;
mod Path;
mod Proxies;
mod Tracing;

pub use Compression::Compression;
pub use Load::Load;
pub use Path::Resolve;
pub use Proxies::Assemble;
pub use Tracing::Subscribe;
