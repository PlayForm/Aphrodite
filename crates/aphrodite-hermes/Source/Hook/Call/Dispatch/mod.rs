//! The hook-name match: one file per arm.
//!
//! The dispatcher itself lives in `Fn.rs` and does nothing but route; each arm
//! owns the whole body for its hook, so a change to one hook's contract cannot
//! touch another's.

pub mod Fn;
pub mod PostLLM;
pub mod PreLLM;
pub mod PreTool;
pub mod Start;
pub mod Terminal;
pub mod ToolResult;
