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
//! This file is the crate root and nothing else: crate docs, the three crate-wide
//! lint gates, and the module list. There are no re-exports - a consumer names
//! the leaf it means (`Session::With::Fn::with_session`), which is the whole point
//! of the one-item-per-file layout.
//!
//! Module map:
//!   - `ABI/` - one file per exported `aphrodite_hermes_*` C symbol.
//!   - `Hook/` - the hook bodies `ABI::CallHook` dispatches into.
//!   - `Session/` - the per-session state registry, its bounded lock, and the
//!     opt-in config watcher with its handle/readiness/shutdown lifecycle.
//!   - `Configuration/` - the config-seeded template and its reload path.
//!   - `Health/` - the proxy port pair, its liveness probe, and the build stamp.
//!   - `Guard/` - the panic guard, the one JSON renderer, and C-string
//!     marshalling every export uses.
//!   - `Tool/`, `Schema/`, `Directive/`, `Debug/` - tool dispatch, tool
//!     schemas, directive provisioning, and the per-session debug flag.
//!   - `Bin/` - the `cargo install` helper binary (a separate target, not a
//!     module of this crate).

// See crates/aphrodite/Source/Library.rs's matching comment: fixing this properly
// (marking every `pub extern "C" fn` `unsafe`) is report 03's job, not a
// side effect of wiring up a CI clippy gate.
#![allow(clippy::not_unsafe_ptr_arg_deref)]
// House style: module directories and item names are PascalCase
// (`Source/Schema/mod.rs`, `Struct`, `Fn`). The convention is the contract,
// so the snake_case module lint is switched off crate-wide rather than
// renamed away at every declaration.
#![allow(non_snake_case)]
// House style: acronyms stay uppercase inside identifiers (`JSON`, `ID`, `URL`,
// `ABI`) - `JSON` is the name, `Json` is a spelling of it. That is the opposite
// of `upper_case_acronyms`' default, so the lint is switched off crate-wide
// rather than spelling acronyms down at every declaration.
#![allow(clippy::upper_case_acronyms)]
// A two-branch decision is a value, not a branch; see `aphrodite`'s `Library.rs`.
#![allow(clippy::obfuscated_if_else)]

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

// The C ABI surface lives under `ABI/`, one `aphrodite_hermes_*` symbol per
// file, and is NOT re-exported here: the symbol names are the contract asserted
// by cbindgen -> `plugins/aphrodite/_bindings.py` and
// `Maintain/check_ffi_contract.py`, and a second spelling of them at the crate
// root would be a second thing to keep in step. Every peer module names the leaf
// it means, in full.
