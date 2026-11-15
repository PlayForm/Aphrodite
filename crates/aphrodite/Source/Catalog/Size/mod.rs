//! Human-readable byte sizes.
//!
//! `Classify/` decides the unit, `Unit/` names it, `Fn/` renders it, and `Kilobyte/`
//! and `Megabyte/` are the two thresholds the classifier compares against. The
//! thresholds are leaves because the renderer divides by them: one number cannot be
//! the comparison floor in one place and a different divisor in another.

pub mod Classify;
pub mod Fn;
pub mod Kilobyte;
pub mod Megabyte;
pub mod Unit;
