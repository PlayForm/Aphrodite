//! Catalog generation - port of plugins/aphrodite/_hooks/catalog.py
//!
//! Builds compression catalog with by-type grouping, total savings,
//! conversation turns, referenced files. Supports toc (compact) and full modes.
//!
//! Two surfaces, one state: `Build/` emits the JSON the `AphroditeCatalog`
//! tool and the C ABI return, `Format/` emits the markdown table an agent
//! reads inline, and `Size/` is the byte rendering both of them print. The
//! table's columns and the JSON's keys are consumed outside this crate, so
//! they are a wire contract, not presentation.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Catalog::Build::Fn::Fn`), so the file that defines an item is the only
//! place it is defined and there is exactly one way to spell it.

pub mod Build;
pub mod Format;
pub mod Size;

#[cfg(test)]
mod Test;
