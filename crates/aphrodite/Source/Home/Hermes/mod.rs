//! The Hermes home - the parent of the plugin loader and of every file Hermes
//! itself owns.
//!
//! Two answers, one file each: the degraded form (`Fn/`, `.` when nothing
//! resolves) and the honest form (`Opt/`, `None` when nothing resolves). The
//! split exists so a caller that must fail loudly - `aphrodite setup` writing
//! into `plugins/aphrodite` - can tell "no home" apart from "home is `.`".

pub mod Fn;
pub mod Opt;
