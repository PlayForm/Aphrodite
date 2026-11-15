//! The process-global handle table.
//!
//! `Struct` owns every piece of the table's policy: when it is initialised, how
//! an id is allocated, what a collision means, how a poisoned lock is recovered,
//! and how a lookup resolves. Consumers call `Allocate`, `Resolve` or `Remove`
//! and never see the `Option`, the `HashMap` or the guard - the `Option` in
//! particular existed only so the static could stay `const`, and every consumer
//! that had to unwrap it was paying for a representation decision it did not
//! make.

pub mod Allocate;
pub mod Const;
pub mod New;
pub mod Remove;
pub mod Resolve;
pub mod Struct;
pub mod With;
