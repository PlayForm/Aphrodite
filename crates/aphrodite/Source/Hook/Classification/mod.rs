//! Type resolution: the reported type, its preview source, and its provenance.
//!
//! One entry point (`Fn.rs`) over three leaves that can each answer on their
//! own - `Detect/` asks the content detector, `Hinted/` recognises the terminal
//! shape, `Refine/` upgrades a generic answer. `Source/` is the vocabulary they
//! report in and `Struct.rs` the value they pass.

pub mod Detect;
pub mod Fn;
pub mod Hinted;
pub mod Refine;
pub mod Source;
pub mod Struct;
