//! The tool entry point: name plus JSON args in, JSON result out.
//!
//! Also owns `ToolHandler`, the signature every registry family returns. Keeping
//! the type here rather than in `Registry/` means a family can be written - and
//! type-checked - without the table that assembles it existing yet.

pub mod Fn;

