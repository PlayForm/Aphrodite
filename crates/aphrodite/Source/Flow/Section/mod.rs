//! How the assembler names, joins and measures a section.
//!
//! Four small questions, four leaves: `Struct.rs` is the section itself (its text
//! and its protection), `Protection/` is the vocabulary that says whether the
//! budget may drop it, `Label/` names a section for the debug line, `Join/`
//! produces the exact string the model receives, and `Size/` measures that same
//! string without building it.

pub mod Join;
pub mod Label;
pub mod Protection;
pub mod Size;
pub mod Struct;
