//! Tool-call metadata as Hermes ships it.
//!
//! One item: the borrowed view of the per-call signals (`Status`, the two
//! error fields, `Args`, `DurationMs`) that the tool-result entry points
//! thread into the `Event::Recent` telemetry ring. The module exists so those
//! signals stop being dropped on the floor without copying any of the JSON
//! they point into.

pub mod Struct;
