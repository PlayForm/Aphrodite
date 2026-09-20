//! CCR marker resolution - port of plugins/aphrodite/_resolve.py
//!
//! Resolves CCR markers to their original content. Supports:
//! - Single hash resolution (inline store only)
//! - Recursive nested marker unpacking (up to RECURSIVE_DEPTH levels)
//! - Query filtering on resolved content
//! - Cycle-safe recursive expansion with resolved cache

mod expand;
mod filter;
mod one;
mod parse;
mod recursive;

#[cfg(test)]
mod tests;

pub use expand::expand;
pub use filter::filter_lines;
pub use one::resolve_one;
pub use recursive::resolve_recursive;
// Re-exported (crate-wide, test target only) so `use super::*` in `tests`
// and the recursive resolver can reach these marker-parsing helpers.
#[cfg(test)]
pub(crate) use parse::{find_markers, parse_marker_hash};

// Imported here so the `use super::*` in `tests` resolves AphroditeState.
#[cfg(test)]
pub(crate) use crate::state::AphroditeState;
