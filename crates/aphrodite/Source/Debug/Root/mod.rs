//! The root session id - what the log file is named after.
//!
//! One static (`Const/`), one writer (`Note/`) and one reader (`Current/`). The
//! id is published by the Hermes bridge and persisted by it too, which is why
//! the reader falls back to the `session.current` file: a fresh process starts
//! with the static empty and must still target the same per-session log.

pub mod Const;
pub mod Current;
pub mod Note;
