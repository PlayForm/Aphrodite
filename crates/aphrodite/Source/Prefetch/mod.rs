//! Prefetch - background file loading into CCR.
//! Port of plugins/aphrodite/_hooks/prefetch.py
//!
//! Agent-agnostic: any agent can preload files into the compression store
//! before they're needed, avoiding round-trips during critical paths.
//!
//! The module is split along the lock boundary that gives it its value:
//! `Read/` does all the I/O with no state access, `Insert/` mutates state
//! with no I/O, and `Files/` composes the two for callers that already hold
//! the lock. `Arrival/` and `Clock/` carry the live-container arrival model,
//! `Status/` renders the schedule, and `Outcome/` is the vocabulary the two
//! halves exchange.
//!
//! This facade only declares. A caller names the leaf
//! (`Prefetch::Files::Fn::prefetch_files`), so the file that defines an item
//! is the only place it is defined.

pub mod Arrival;
pub mod Clock;
pub mod Files;
pub mod Insert;
pub mod Outcome;
pub mod Read;
pub mod Status;

#[cfg(test)]
mod Test;
