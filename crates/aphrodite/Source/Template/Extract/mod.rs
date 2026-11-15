//! The content extractors.
//!
//! One leaf per marker the pre-Rust engine recognised: `Exit/` (the house `EXIT:<digits>`
//! trace), `Command/` (a `$`/`>` prefixed line), `Commit/` (a git hash) and `Diff/` (the
//! `+`/`-` tallies).
//!
//! They are heuristics over the FIRST match in the content, and each one answers `None` for
//! "not found" instead of the pre-Rust placeholder: `"?"`, `""` and `"???????"` are
//! RENDERINGS of absence, applied once in `Vars::Derive` where the template namespace is
//! defined. An extractor that spelled the placeholder itself made "no commit in this
//! content" indistinguishable from a commit whose hash is literally seven question marks.
//!
//! Keeping them separate makes it obvious that adding a new marker is a new leaf rather than
//! another branch in a growing scanner.

pub mod Command;
pub mod Commit;
pub mod Diff;
pub mod Exit;
