//! Tool-result compression.
//!
//! `Entry/` is the public surface (three signatures over one pipeline), `Inner/`
//! is the pipeline itself, and the tree is its three steps: `Observation/`
//! (shared with the terminal path) records the call, `Input/` + `Policy/` +
//! `Select/` + `Plan/` decide what to do with it, and `Apply/` + `Emit/` do it.
//!
//! The payload shape produced here is what the Hermes bridge hands to the model,
//! so every field is a contract.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Hook::ToolResult::Inner::Fn::Fn`), so the file that defines an item is the
//! only place it is defined.

pub mod Apply;
pub mod Emit;
pub mod Entry;
pub mod Inner;
pub mod Input;
pub mod Plan;
pub mod Policy;
pub mod Select;
