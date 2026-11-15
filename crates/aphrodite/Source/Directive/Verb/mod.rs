//! The directive action vocabulary.
//!
//! One leaf per verb, because each verb has its own refusal rule and its own reply shape:
//! `swap` refuses an unknown name, `load` refuses and reports `loaded`, `add` is silent,
//! `remove` is idempotent, `list` reports and `reset` clears. `Action/` is the dispatch and
//! nothing else, so the vocabulary cannot drift from the leaves.
//!
//! `Push/` is the "activate unless already active" step `add` and `load` share, and
//! `Unknown/` is the one spelling of the refusal both of them answer.

pub mod Add;
pub mod List;
pub mod Load;
pub mod Push;
pub mod Remove;
pub mod Reset;
pub mod Swap;
pub mod Unknown;
