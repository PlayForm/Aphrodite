//! The embedded Hermes plugin loader (`__init__.py`).
//!
//! Setup installs the same Python loader the monorepo ships as the live plugin,
//! compiled in, rather than maintaining a second hand-copied template: the
//! hand-maintained copy had drifted into stale dylib reload/free handling, a
//! `register_tool` call with the wrong argument count, no skills registration, no
//! version handshake, no port-environment reads, and stderr piped to `DEVNULL`.
//! The embedded file is `crates/aphrodite/templates/__init__.py`, a byte copy of
//! `plugins/aphrodite/__init__.py`; `Setup/Test/PluginShim.rs` fails if the two
//! diverge.

#![allow(non_upper_case_globals)]

/// The Hermes plugin loader, compiled in.
///
/// Written verbatim into the plugin directory on every run. It is the whole of
/// what setup installs into the Hermes home - the loader registers hooks and
/// tools and resolves every runtime artifact from the runtime home.
///
/// It is not a template: no placeholder is substituted, so the bytes written are
/// the bytes compiled in. Editing this constant, or the file it embeds, without
/// re-copying the live plugin installs a loader that disagrees with the plugin the
/// repository documents - which is exactly the drift the drift guard exists to
/// catch.
pub(crate) const Const:&str = include_str!("../../../templates/__init__.py");
