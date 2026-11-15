//! aphrodite-hermes: Hermes Agent-specific integration crate.
//!
//! This crate handles all Hermes-specific concerns - tool schemas,
//! hook dispatch, directive provisioning - leaving the core `aphrodite`
//! crate as a pure, agent-agnostic compression engine. (Bundled skill
//! registration is the Python plugin's job, not this crate's.)
//!
//! Architecture:
//!   Python plugin (thin loader) → ctypes → libaphrodite_hermes.dylib
//!                                           ├─ Tool dispatch (compress, retrieve, stats, etc.)
//!                                           ├─ Hook dispatch (on_session_start, transform, terminal, pre/post LLM)
//!                                           ├─ Directive provisioning (materialize the
//!                                           │  embedded builtin set into the runtime home)
//!                                           ↓ (depends on)
//!                                    aphrodite crate (rlib)
//!                                           ├─ Core compression (hooks, state, marker)
//!                                           ├─ Resolution (resolve, stage2, struct)
//!                                           └─ Catalog, session, prefetch, config, chain_split
//!
//! This file is the crate root and nothing else: crate docs, the two crate-wide
//! lint gates, the module list, and the re-exports that keep every caller's
//! existing path working while the tree is atomized.
//!
//! Module map:
//!   - `ABI/` - one file per exported `aphrodite_hermes_*` C symbol.
//!   - `Hook/` - the hook bodies `ABI::CallHook` dispatches into.
//!   - `Session/` - the per-session state registry and its bounded lock.
//!   - `Configuration/` - the config-seeded template and its reload path.
//!   - `Health/` - the proxy port pair and its liveness probe.
//!   - `Guard/` - the panic guard and C-string marshalling every export uses.
//!   - `Tool/`, `Schema/`, `Directive/`, `Debug/` - tool dispatch, tool
//!     schemas, directive provisioning, and the per-session debug flag.

// See crates/aphrodite/Source/Library.rs's matching comment: fixing this properly
// (marking every `pub extern "C" fn` `unsafe`) is report 03's job, not a
// side effect of wiring up a CI clippy gate.
#![allow(clippy::not_unsafe_ptr_arg_deref)]
// House style: module directories and item names are PascalCase
// (`Source/Schema/mod.rs`, `Struct`, `Fn`). The convention is the contract,
// so the snake_case module lint is switched off crate-wide rather than
// renamed away at every declaration.
#![allow(non_snake_case)]

mod ABI;
mod Configuration;
mod Debug;
mod Directive;
mod Guard;
mod Health;
mod Hook;
mod Schema;
mod Session;
mod Tool;

// Re-export core aphrodite types for convenience
pub use aphrodite::State::AphroditeState;
// The C ABI surface: every `aphrodite_hermes_*` symbol Hermes binds, one per
// file under `ABI/`. Re-exported at the crate root so the exported symbol set
// stays greppable from one place; the symbol NAMES are the contract asserted by
// cbindgen -> `plugins/aphrodite/_bindings.py` and
// `Maintain/check_ffi_contract.py`, and none of them may change.
pub use ABI::{
	CallHook::aphrodite_hermes_call_hook,
	DispatchTool::aphrodite_hermes_dispatch_tool,
	FreeString::aphrodite_hermes_free_string,
	GetHooks::aphrodite_hermes_get_hooks,
	GetSchema::aphrodite_hermes_get_schema,
	GetSchemas::aphrodite_hermes_get_schemas,
	ListTools::aphrodite_hermes_list_tools,
	MaterializeDirectives::aphrodite_hermes_materialize_directives,
	ProxyHealth::aphrodite_hermes_proxy_health,
	Version::aphrodite_hermes_version,
};
// Crate-internal names that peer modules (`Tool/`, `Directive/`, `Schema/`)
// reach through the crate root.
pub(crate) use Health::Proxy::Fn::proxy_health;
pub(crate) use Session::With::Fn::with_shared;
// Only the session-isolation probe reaches the crate root for `with_session`:
// production code paths go straight to `Session::With::Fn`, so the alias is
// gated to keep the lib build warning-free.
#[cfg(test)]
pub(crate) use Session::With::Fn::with_session;
#[cfg(test)]
pub(crate) use Session::Guard::Fn::test_guard;
