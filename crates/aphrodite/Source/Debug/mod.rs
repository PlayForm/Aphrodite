//! DEV-ONLY env-gated debug logger (TEMPORARY development instrumentation).
//!
//! WARNING: this module is temporary dev-only compute. It is gated behind
//! `APHRODITE_DEBUG` (read into `AphroditeState::debug_level` by
//! `ConfigurationLoader::apply_compression` at session start) and must NEVER be
//! relied on in a release path. When the level is 0 every call site is a
//! no-op: the enabled check is one relaxed atomic load and no formatting,
//! file I/O, or timing runs (byte-identical behavior - existing tests stay
//! green).
//!
//! When enabled (1 = on, 2 = verbose), timestamped single-line structured
//! records are APPENDED to `debug.<root-session-id>` under the runtime home
//! (`~/.hermes/aphrodite/`; $APHRODITE_HOME -> $HERMES_HOME -> $HOME ->
//! platform default - the same `Home::Runtime::Fn::Fn` decision the flag-file
//! toggle in `aphrodite-hermes/src/debug.rs` uses). The root session id is
//! published by the Hermes bridge (`note_root_session`, fed from its
//! `record_session`) and persisted across processes via the `session.current`
//! file, so a fresh Hermes process still targets the same per-session log.
//!
//! The `debug.<root>` file doubles as the legacy flag file: the flag reader
//! (`aphrodite-hermes` `flag_enabled`) resolves the FIRST line only, so an
//! appended log history never flips the toggle, and a toggle write ("on" /
//! "off", whole-file overwrite) still reads back correctly. Appended lines
//! are the logger channel; the first line decides the flag.
//!
//! Every write is best-effort: a failed append (missing dir, permissions)
//! degrades to silence - a debug-logging failure must never break a
//! compression or turn.
//!
//! Every record ALSO enters the process-wide `tracing` stack
//! (`Debug::Log::Emit`, target `aphrodite::debug`, level `DEBUG`), so one
//! stream carries both the proxy's spans and the engine's per-turn decisions.
//! The file append stays direct instead of becoming a `tracing` layer, and that
//! is deliberate: the file's bytes are a documented contract that has to
//! survive a process where no subscriber is installed at all - the Hermes dylib
//! before `AphroditeInit`, or a host that installed its own dispatcher first -
//! and a layer would only write when a subscriber it does not own exists.
//!
//! The tree is grouped by WHAT a record describes: `Level/` is the gate,
//! `Log/` is the append itself (record shape, kv sanitiser, verbose variant),
//! `Count/` and `Timing/` are the accumulators, `Root/` is the session id the
//! file is named after, `Path/` is where the file lives, and `Engine/`,
//! `Metrics/`, `Summary/`, `Time/` are the four record kinds the engine emits.
//!
//! The accumulator directory is `Count/`, not `Counter/`: `Debug::Count::Enum::Enum`
//! must stay the ENUM that call sites write variants through, and a module of
//! that name would take the path away (a module and a type share one namespace).
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Debug::Log::Fn::Fn`), so the file that defines an item is the only place
//! it is defined and there is exactly one way to spell it.

pub mod Count;
pub mod Engine;
pub mod Level;
pub mod Log;
pub mod Metrics;
pub mod Path;
pub mod Root;
pub mod Summary;
pub mod Time;
pub mod Timing;

#[cfg(test)]
mod Test;
