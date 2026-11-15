//! The flag files that carry the per-session debug switch.
//!
//! An env var cannot be used: the gateway process env is frozen at launch, so a
//! mid-session toggle has to be an artifact on disk. This module owns the
//! naming (`FLAG_PREFIX`), the two paths (scoped and global), the cached read,
//! and the resolution that walks the session tree.
//!
//! The path and read children are reachable crate-wide because the session and
//! log modules both address a flag file directly - resolution is not the only
//! reader.

pub(crate) mod Global;
pub(crate) mod Read;
pub(crate) mod Resolve;
pub(crate) mod Scoped;

mod Cache;
mod Const;

pub(crate) use Cache::FLAG_CACHE;
pub(crate) use Const::FLAG_PREFIX;
