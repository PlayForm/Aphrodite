//! The state one expansion owns for its own duration.
//!
//! The walk needs three things to be safe and bounded - what it has already loaded,
//! where it currently is, and what it is allowed to spend - and all three belong to
//! ONE expansion. Holding them in a type (rather than passing two mutable collections
//! down every call) is what makes "this guard cannot leak into the next retrieval" a
//! property of the code instead of a convention.

pub mod Struct;
