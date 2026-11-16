//! Writing resolved configuration into runtime state.
//!
//! Two passes, one per lifecycle: `Startup` applies every section to a fresh
//! session, `Reload` applies every section a live session has not taken ownership of
//! (all of them but `[directives]`). The distinction is the authority each pass
//! holds, and it is expressed by the applier list it runs rather than by a flag.
//!
//! `Section` holds one applier per configuration section, and `Startup`/`Reload` hold
//! the ordered lists. The split exists so a section can be read, changed and tested
//! without touching the other seven - it is not licence to reorder them, because a few
//! sections seed values the next one reads (`ChainSplit::Floor` from
//! `ChainSplit::Minimum`, `Directive::Active` from the loaded set).
//!
//! `Global` is separate because those appliers do not write state at all - they
//! write process globals that paths without an `AphroditeState` (hooks, the dylib,
//! the poll worker) read.

pub mod Global;
pub mod Reload;
pub mod Section;
pub mod Startup;
