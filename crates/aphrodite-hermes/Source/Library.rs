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

pub mod ABI;
pub mod Configuration;
pub mod Debug;
pub mod Directive;
pub mod Guard;
pub mod Health;
pub mod Hook;
pub mod Schema;
pub mod Session;
pub mod Tool;

// Re-export core aphrodite types for convenience

// The C ABI surface: every `aphrodite_hermes_*` symbol Hermes binds, one per
// file under `ABI/`. Re-exported at the crate root so the exported symbol set
// stays greppable from one place; the symbol NAMES are the contract asserted by
// cbindgen -> `plugins/aphrodite/_bindings.py` and
// `Maintain/check_ffi_contract.py`, and none of them may change.

// Crate-internal names that peer modules (`Tool/`, `Directive/`, `Schema/`)
// reach through the crate root.

// Only the session-isolation probe reaches the crate root for `with_session`:
// production code paths go straight to `Session::With::Fn`, so the alias is
// gated to keep the lib build warning-free.

