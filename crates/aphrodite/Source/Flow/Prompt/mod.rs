//! Prompt templates and their renderer.
//!
//! Two halves, deliberately apart: `Const` owns the compiled-in strings,
//! `Render` owns the order that picks between a configured template and those
//! strings. Keeping the text out of the renderer is what makes "the rendered
//! prompt text is the product" auditable - a change to either half is a change
//! to what the model reads.

pub mod Const;
pub mod Render;

