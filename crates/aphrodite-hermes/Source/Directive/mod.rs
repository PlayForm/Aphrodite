//! Materialization of the binary-provided behavioral directives into the
//! runtime home.
//!
//! The shipped directive set is EMBEDDED in the core `aphrodite` crate
//! (`crates/aphrodite/Source/BuiltinDirective/*.md`, compiled in via
//! `include_str!` and exposed through `aphrodite::Directive::Builtin::Fn()`).
//! This module provisions those builtins into the user-data home
//! (`~/.hermes/aphrodite/directives/` by default) so the plugin directory
//! never has to hold runtime state: the core crate is the provider (the
//! embedded set ships inside this dylib), the runtime home is the store,
//! and the core directive loader reads the same location
//! (its home-namespace candidate, or `$APHRODITE_DIRECTIVES_DIR` when set).
//!
//! Provisioning is idempotent and strictly non-destructive: files are only
//! ever written when missing; a pre-existing file that differs from the
//! embedded bytes is left untouched (user data wins). Files are written as
//! `<name>.md` (the core loader only reads `*.md` files and derives the
//! directive name from the file stem).
//!
//! Layout: `Resolve/` decides WHERE the set lands, `Builtin/` decides WHAT
//! the set is, `Materialize/` writes it, `Into/` joins the three for the C
//! ABI export. Each leaf owns one item named by its kind; this facade only
//! declares children and re-exports the item the FFI export calls.

mod Builtin;
pub(crate) mod Into;
mod Materialize;
mod Resolve;

#[cfg(test)]
mod Test;
