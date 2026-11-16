//! Code structure extraction - the signature preview the model reads.
//!
//! A stored code blob is previewed as `[code_rust:3fns 2structs]`, and that line is produced here.
//! It is a product surface: the categories it reports are the only structural information the model
//! gets about the file, so a signature that overruns the budget silently drops the categories behind
//! it and the model then reasons about a file it was never shown.
//!
//! The tree is one recogniser per declaration shape (`Recognize/<Language>/<Shape>`), one renderer
//! (`Render`), one admission policy (`Policy`), one summary (`Summary`), one scanner (`Scan`) and one
//! budget (`Budget`). A recogniser answers "is this line a declaration, and if so of what" - it owns
//! no truncation, no budget and no category key. That split is what makes the category priority order
//! a readable list instead of an accident of statement order inside four near-identical scanners.
//!
//! The priority order is load-bearing and lives in `Policy`: Rust pays for functions, then structs,
//! then traits, then impls out of one budget, and a file with many functions therefore reports no
//! traits at all rather than a fraction of each. A one-pass source-order rewrite would change which
//! declarations survive.
//!
//! This facade only declares the tree.

pub mod Budget;
pub mod Declaration;
pub mod Detect;
pub mod Extract;
pub mod Format;
pub mod Maximum;
pub mod Policy;
pub mod Recognize;
pub mod Render;
pub mod Scan;
pub mod Summary;

#[cfg(test)]
mod Test;
