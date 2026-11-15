//! The pre-subscriber sink.

use std::io::Write;

/// Write one diagnostic line to stderr, without a subscriber.
///
/// Public because the binary's own pre-configuration paths (`Boot::Fn`) need
/// the same sink the library's config loader uses; it is not an invitation to
/// log through stderr, and it must not grow into a second stack. It exists for
/// the messages produced before `Logging::Fn` can run - a config parse failure
/// during startup, a malformed `APHRODITE_WORKER_THREADS`, a failed `setup` -
/// and for a host process that already owns the global dispatcher, where a
/// `tracing` event would be swallowed. Routed through `tracing::warn!` those
/// messages would be a silent no-op, which is exactly the failure the config
/// loader's fallback was added to fix.
///
/// Write errors are ignored on purpose: a logger that panics because stderr is
/// closed turns a diagnostic into an outage.
pub fn Fn(Message:&str) { let _ = writeln!(std::io::stderr(), "{Message}"); }
