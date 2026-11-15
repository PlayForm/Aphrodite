//! Rolling-window aggregates over recorded tool events.

pub mod Stats;
pub mod Turn;

pub use self::{Stats::Struct::WindowStats, Turn::Fn::TurnWindow};
