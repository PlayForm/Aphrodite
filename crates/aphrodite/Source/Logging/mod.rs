//! The one logging stack: a `tracing` subscriber with an `EnvFilter`.
//!
//! Every level of this crate - the binary, the C ABI dylib, the proxy, the
//! hook pipeline - emits through `tracing` and nothing else. There is one
//! subscriber per process and `Fn` installs it. A second call is a no-op,
//! which is what makes it safe for the binary and the dylib to both call it in
//! one process: `tracing` allows exactly one global dispatcher, and a second
//! `try_init` would otherwise turn a shared process into a startup error.
//!
//! The filter chain is `APHRODITE_LOG`, then `RUST_LOG`, then the level the
//! caller passes from its own configuration, then `info`. `APHRODITE_LOG` is
//! first because a product-specific variable must beat a shell-wide `RUST_LOG`
//! a user forgot about; `info` is last because silence is never an acceptable
//! default for a proxy - a process that logs nothing is indistinguishable from
//! a process that does nothing.
//!
//! `Stderr/` is not a second stack. It is the sink for the few messages that
//! must land BEFORE any subscriber exists (a config parse failure during
//! startup, a malformed worker-thread count, a failed `setup`), where
//! `tracing` would drop every one of them on the floor.
//!
//! The DEV-ONLY `Debug/` tree is deliberately not part of this module. It owns
//! the `APHRODITE_DEBUG` gate and the per-session `debug.<root>` file whose
//! bytes are a documented contract, and it feeds this stack as well
//! (`Debug::Log::Emit`) while keeping its file append direct - see that
//! module for why.

pub mod Fn;
pub mod Install;
pub mod Installed;
pub mod Level;
pub mod Stderr;
