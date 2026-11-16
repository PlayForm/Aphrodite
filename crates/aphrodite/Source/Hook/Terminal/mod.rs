//! Terminal-output compression.
//!
//! The same store-and-marker contract as tool results, with two differences the
//! shape of terminal output forces: marked output (chain-split segments produced
//! by a shell script) is split before any threshold gate can let it leak raw, and
//! a failing command is observed even when its output is tiny.
//!
//! The tree is the three steps of one pipeline: `Observation/` (shared with the
//! tool path) records the call, `Input/` + `Policy/` + `Select/` + `Plan/` decide
//! what to do with it, and `Apply/` does it - through the shared `Hook::Emit/`
//! leaf, which both transform paths use so their payload contract cannot fork.
//! `Entry/` is the public surface and `Inner/` is the three steps in order.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Hook::Terminal::Inner::Fn::Fn`), so the file that defines an item is the only
//! place it is defined.

pub mod Apply;
pub mod Entry;
pub mod Inner;
pub mod Input;
pub mod Plan;
pub mod Policy;
pub mod Select;
