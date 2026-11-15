//! The cumulative counters.
//!
//! `Enum/` names the slots, `Const/` is the storage those slots index, and
//! `Bump/` is the only writer. The directory is `Count/` rather than
//! `Counter/` because `Debug::Count::Enum::Enum` has to stay the enum itself - see the
//! module docs on `Debug`.

pub mod Bump;
pub mod Const;
pub mod Enum;
