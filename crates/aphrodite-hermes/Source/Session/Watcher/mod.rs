//! The opt-in config auto-reload watcher thread.
//!
//! The THREAD is runtime-only: it spawns from `Watcher::Fn`, which
//! `Configuration::Template::Seeded` calls when the loaded config asks for
//! auto-reload, and a test must never start a background thread watching the
//! repo's live `aphrodite.toml`. Everything the thread drives is not runtime-only
//! - `Debounce/` collapses an event burst from a plain channel, `Wait/` names the
//!   three ways the loop can wake, `Apply/` applies a supplied config to the
//!   registry, and `Continue/` decides whether to keep watching - so all of those
//!   stay compiled in the test build and are exercised there.
//!
//! Lifecycle: `Fn/` spawns and parks a `Handle/` in `State/`, `Run/` is the
//! thread body, and `Ready/` is the readiness report the spawner waits for. The
//! thread is joined, never detached: this is a dylib, and Hermes hot-reloads it
//! on mtime change, so a thread of this image must be stoppable before the image
//! can go away.

pub mod Apply;
pub mod Continue;
pub mod Debounce;
pub mod Event;
pub mod Outcome;
pub mod Select;
pub mod Wait;

#[cfg(not(test))]
pub mod Fn;
#[cfg(not(test))]
pub mod Handle;
#[cfg(not(test))]
pub mod Ready;
#[cfg(not(test))]
pub mod Run;
#[cfg(not(test))]
pub mod Source;
#[cfg(not(test))]
pub mod Start;
#[cfg(not(test))]
pub mod State;
