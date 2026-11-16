//! aphrodite: core compression engine exposed as both an rlib (for the
//! `Aphrodite` binary and the `aphrodite-hermes` bridge) and a C ABI cdylib.
//!
//! The C ABI is agent-agnostic and handle-based: `AphroditeInit` allocates a
//! session handle, the `aphrodite_*` functions operate on it (classify,
//! compress, retrieve, transform, terminal, session_start, catalog, stats,
//! reload, config_get/set, search, dispatch, resolve, stage2, struct_extract),
//! and `AphroditeDestroy` frees it. The Hermes plugin uses the higher-level,
//! process-global `aphrodite_hermes_*` ABI in the `aphrodite-hermes` crate.
//!
//! This file is the crate root and nothing else: crate docs, the three lint gates
//! the house style needs, the module declarations, and the `Preview`
//! re-exports. Every `aphrodite_*` export lives in its own file under `ABI/`,
//! one symbol per file, because the symbol name is the contract that cbindgen,
//! the generated `_bindings.py` and `Maintain/check_ffi_contract.py` assert.

// House style: module directories and item names are PascalCase
// (`Source/Preview/Detector/mod.rs`, `Struct`, `Fn`). The convention is the
// contract, so the snake_case module lint is switched off crate-wide rather
// than renamed away at every declaration.
#![allow(non_snake_case)]
// House style: constants and statics are PascalCase (`const Maximum`,
// `static HandleTable`), the same convention as every other item. That is the
// opposite of `non_upper_case_globals`' default, so the lint is switched off
// crate-wide rather than silenced file by file at every declaration.
#![allow(non_upper_case_globals)]
// House style: acronyms stay uppercase inside identifiers (`JSON`, `ID`, `URL`,
// `ABI`) - `JSON` is the name, `Json` is a spelling of it. That is the opposite
// of `upper_case_acronyms`' default, so the lint is switched off crate-wide
// rather than spelling acronyms down at every declaration.
#![allow(clippy::upper_case_acronyms)]
// House style: a two-branch decision is a value, not a branch - the condition
// carries the value (`Cond.then_some(A).unwrap_or(B)`), so the call site reads
// as one expression instead of a statement with two arms. That is the opposite
// of `obfuscated_if_else`'s default, which asks for the `if .. else ..` back,
// so the lint is switched off crate-wide rather than re-branching every site.
#![allow(clippy::obfuscated_if_else)]

pub mod ABI;
pub mod Catalog;
pub mod ChainSplit;
pub mod ConfigurationLoader;
/// DEV-ONLY env-gated debug logger (temporary development instrumentation,
/// gated behind `APHRODITE_DEBUG` - never a release path).
pub mod Debug;
pub mod Directive;
pub mod Home;
pub mod Flow;
pub mod Hook;
/// The one `tracing` subscriber, installed once per process.
pub mod Logging;
pub mod Marker;
pub mod PollWorker;
pub mod Prefetch;
pub mod Preview;
pub mod Resolve;
pub mod Session;
pub mod Stage2;
pub mod State;
pub mod StructExtract;
pub mod Template;

// Proxy modules (used by main.rs binary) - gated behind the `Proxy`
// feature (report 01-T2) so the cdylib built by `aphrodite-hermes` with
// `default-features = false` doesn't link axum/tokio/reqwest into the
// Hermes dylib unnecessarily.
// center.rs removed (report 01-T6): zero callers, dead machinery
#[cfg(feature = "proxy")]
pub mod Configuration;
#[cfg(feature = "proxy")]
pub mod Proxy;
#[cfg(feature = "proxy")]
pub mod Retrieve;
#[cfg(feature = "proxy")]
pub mod Setup;

// Rhai scripting engine (pre-Rust scripting.rs port): user-defined
// micro-scripts that transform hook payloads. Feature-gated so the default
// cdylib build doesn't link rhai.
#[cfg(feature = "scripting")]
pub mod Scripting;

// ── Preview builder ──────────────────────────────────────────────────
//
// The builder lives at `crate::Preview` (report 01-T11). There is no
// compatibility alias: a consumer names the leaf path
// (`crate::Preview::Builder::Entry::Fn`) directly, which is the same rule every
// other module in this crate follows.
