//! The content extractors.
//!
//! One leaf per marker the pre-Rust engine recognised: `Exit/` (the house
//! `EXIT:<digits>` trace), `Command/` (a `$`/`>` prefixed line), `Commit/` (a
//! git hash) and `Diff/` (the `+`/`-` tallies).
//!
//! They are heuristics over the FIRST match in the content, and each one has a
//! pre-Rust default for "not found" (`"?"`, `""`, `"???????"`). Keeping them
//! separate makes it obvious that adding a new marker is a new leaf rather than
//! another branch in a growing scanner.

pub mod Command;
pub mod Commit;
pub mod Diff;
pub mod Exit;
