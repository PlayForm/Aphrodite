//! Session lifecycle - port of `plugins/aphrodite/_hooks/session.py`.
//!
//! Manages the turn counter, the conversation index, the referenced-file list,
//! the message scan cursor and the marker-at-end-of-turn archive. The state is
//! process-global and survives a session, so every field here is either reset
//! by `Start` or bounded by the leaf that writes it; a field that is neither
//! leaks one conversation into the next.
//!
//! `Catalog` and `Engine` are the two leaves the Flow assembler reaches into
//! directly: the catalog because it renders a section, the engine because it
//! mutates the history before that section is written. Everything else is
//! lifecycle plumbing called from the hooks. This facade declares and
//! re-exports; it defines nothing.

pub mod Archive;
pub mod Catalog;
pub mod Engine;
pub mod Index;
pub mod Start;
pub mod Turn;

#[cfg(test)]
pub mod Test;

