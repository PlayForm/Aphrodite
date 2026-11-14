//! CCR marker resolution - port of plugins/aphrodite/_resolve.py
//!
//! Resolves CCR markers to their original content. Supports:
//! - Single hash resolution (inline store only)
//! - Recursive nested marker unpacking (up to RECURSIVE_DEPTH levels)
//! - Query filtering on resolved content
//! - Cycle-safe recursive expansion with resolved cache

mod Expand;
mod Filter;
mod One;
mod Parse;
mod Recursive;

#[cfg(test)]
mod Test;

pub use Expand::Expand;
pub use Filter::filter_lines;
pub use One::resolve_one;
pub use Recursive::{resolve_inline, resolve_recursive};
// Re-exported (crate-wide, test target only) so `use super::*` in `tests`
// and the recursive resolver can reach these marker-parsing helpers.
#[cfg(test)]
pub(crate) use Parse::{find_markers, parse_marker_hash};

// Imported here so the `use super::*` in `tests` resolves AphroditeState.
#[cfg(test)]
pub(crate) use crate::State::AphroditeState;
