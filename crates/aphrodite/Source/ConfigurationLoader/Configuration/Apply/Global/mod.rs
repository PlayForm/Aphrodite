//! The appliers that write process globals instead of state.
//!
//! Hooks, the dylib entry points and the poll worker run without an
//! `AphroditeState`, so the preview cap, the marker grammar and the template
//! renderer have to be reachable without one. `Rendering` is what a document says
//! those globals should be; `Publish` is the one place they are written; `Fn` is the
//! convenience entry that builds one from a document.
//!
//! This is deliberately not part of `Apply::Startup`: state appliers are per
//! session, and these are per process. Mixing the two is what let a reload publish
//! the globals zero times or N times depending on which session locks were free.

pub mod Fn;
pub mod Publish;
pub mod Rendering;
