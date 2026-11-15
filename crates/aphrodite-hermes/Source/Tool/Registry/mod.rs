//! The tool table: name -> handler, one leaf per tool family.
//!
//! Families are separate leaves so a tool can be read, tested and reviewed on its
//! own; `Fn/` assembles them. The table is rebuilt per call rather than cached, so
//! a newly added family cannot be missed.

pub(crate) mod Catalog;
pub(crate) mod Compress;
pub(crate) mod ContextEngine;
pub(crate) mod Diff;
pub(crate) mod Directive;
pub(crate) mod File;
pub(crate) mod Prefetch;
pub(crate) mod PrefetchStatus;
pub(crate) mod Rebuild;
pub(crate) mod Reclassify;
pub(crate) mod Retrieve;
pub(crate) mod Search;
pub(crate) mod Stats;
pub(crate) mod Test;

#[cfg(debug_assertions)]
pub(crate) mod Debug;

pub mod Fn;

