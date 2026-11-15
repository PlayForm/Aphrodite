//! The two appliers that write process globals instead of state.
//!
//! Hooks, the dylib entry points and the poll worker run without an
//! `AphroditeState`, so the preview cap and the template renderer have to be
//! reachable without one. These are the dylib-side wiring for exactly that;
//! the engine binary reads the same values off `MultiConfiguration` directly.

pub mod Preview;
pub mod Template;
