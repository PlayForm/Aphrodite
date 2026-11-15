//! Code structure extraction - the signature preview the model reads.
//!
//! A stored code blob is previewed as `[code_rust:3fns 2structs]`, and that line
//! is produced here. It is a product surface: the categories it reports are the
//! only structural information the model gets about the file, so a signature that
//! overruns the budget silently drops the categories behind it and the model then
//! reasons about a file it was never shown.
//!
//! One leaf per language (`Rust`, `Python`, `Go`, `JS`), one for detection
//! (`Detect`), one for the byte-safe truncation they share (`Format`), and one for
//! the entry point that owns the budget and the dispatch table (`Extract`). This
//! facade only declares the tree and keeps the pre-kind names alive for callers
//! outside this module.

pub mod Budget;
pub mod Detect;
pub mod Extract;
pub mod Format;
pub mod Go;
pub mod JS;
pub mod MaximumParametersLength;
pub mod MaximumSignatureLength;
pub mod Python;
pub mod Rust;

#[cfg(test)]
mod Test;
