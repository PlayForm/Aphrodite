//! `impl AppState` method groups, one file per responsibility.
//!
//! The type lives in `State::Struct`; these files add behaviour to it. A method
//! group that needs a new field is a signal to look at the type's own discipline
//! doc before adding one.

pub mod Cache;
pub mod Fill;
pub mod Ratio;
pub mod Record;
pub mod Stats;
pub mod Threshold;
