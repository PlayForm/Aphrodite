//! One file per exported `aphrodite_hermes_*` symbol - the C ABI leaves.
//!
//! Each file owns exactly ONE `#[unsafe(no_mangle)] pub extern "C" fn`, and the
//! item name IS the exported symbol, byte for byte. Renaming one is an ABI
//! break, not a style change: cbindgen generates
//! `plugins/aphrodite/_bindings.py` from these signatures and
//! `Maintain/check_ffi_contract.py` asserts the restype/argcount contract
//! against them, so the house-style "name the item by kind" rule stops at the
//! symbol boundary. The FILE is PascalCase (`DispatchTool.rs`); the SYMBOL keeps
//! its snake_case ABI name.
//!
//! Every export except `FreeString` routes its body through `Guard::guarded`, so
//! a panic becomes error JSON instead of aborting the host process, and every
//! string it returns is allocated by `Guard::to_c_string` and reclaimed only by
//! `FreeString`.

pub mod CallHook;
pub mod DispatchTool;
pub mod FreeString;
pub mod GetHooks;
pub mod GetSchema;
pub mod GetSchemas;
pub mod ListTools;
pub mod MaterializeDirectives;
pub mod ProxyHealth;
pub mod Version;

#[cfg(test)]
mod Test;
