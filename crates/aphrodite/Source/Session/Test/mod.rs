//! Session tests, split by area.
//!
//! Each file owns one behaviour family: start-of-session reset, the turn
//! counter, the conversation index and its eviction, the delta catalog, and
//! the context engine. The engine area carries its own fixtures because its
//! "did the state change at all" assertions are specific to it.

mod Archive;
mod Catalog;
mod Engine;
mod Start;
mod Turn;
