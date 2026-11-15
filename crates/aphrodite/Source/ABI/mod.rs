//! The C ABI surface, one file per exported symbol.
//!
//! Every `aphrodite_*` export in this crate is declared here as its own module,
//! named by the domain the symbol serves (`Compress`, `Reload`, `Stage2`, ...),
//! and every one of those modules defines exactly one `#[unsafe(no_mangle)] pub
//! extern "C" fn` whose identifier is the real symbol. The symbol name is the
//! ABI contract - cbindgen generates `_bindings.py` from it, the Hermes plugin
//! calls it by name, and `Maintain/check_ffi_contract.py` asserts it - so unlike
//! the rest of the crate these leaves keep their exported identifier rather than
//! taking the house-style `Fn` name.
//!
//! Shared machinery sits beside the exports and is not exported itself:
//!
//! - `Boundary` - the one adapter: panic interception, error conversion,
//!   serialisation and ownership transfer. An export body is one fallible
//!   operation and the boundary owns everything else.
//! - `Guard` - the caller-string reader, the payload validator, and the
//!   rejection primitive the validator is built from.
//! - `Handle` - the session registry and the per-session lock.
//! - `Configuration` - the settable keys and the reload assessment, both
//!   parsed outside the session lock and applied inside it.
//! - `Route` - the dispatch routing table, decoded and prepared before the
//!   session lock is taken.
//! - `Macro` - the `stateful!` shape.

// `macro_rules!` is textually scoped: this declaration must stay above the
// modules that expand the macro (`Compress`, `Transform`, `Terminal`), or their
// call sites fail with "cannot find macro `stateful` in this scope".
#[macro_use]
pub(crate) mod Macro;

pub mod Boundary;
pub mod CallHook;
pub mod Catalog;
pub mod Classify;
pub mod Compress;
pub mod Configuration;
pub mod ConfigurationGet;
pub mod ConfigurationSet;
pub mod Destroy;
pub mod Directive;
pub mod Dispatch;
pub mod FilterLines;
pub mod FreeString;
pub mod Guard;
pub mod Handle;
pub mod Hooks;
pub mod Initialise;
pub mod Preview;
pub mod Reload;
pub mod Resolve;
pub mod Retrieve;
pub mod Route;
pub mod Search;
pub mod SessionStart;
pub mod Stage2;
pub mod Stats;
pub mod StructExtract;
pub mod Terminal;
pub mod Transform;
pub mod Version;

#[cfg(test)]
mod Test;
