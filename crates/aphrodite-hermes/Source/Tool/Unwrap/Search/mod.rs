//! The search-envelope shapes: match rows, not a match count.
//!
//! Real Hermes `search_files` results ship either a `matches` array or a
//! path-grouped `matches_text` block, and the preview counts `path:line:content`
//! rows. Both forms are normalised to those rows, which is why a 5,000-hit
//! search previews with the real count instead of `[search:20 hits ...]`.
//!
//! Layout: `Payload/` decides WHICH form arrived and holds the two variants,
//! `Rows/` normalises each form, `Total/` renders the count-only fallback, and
//! this facade composes them. The three outcomes are distinct by construction: a
//! search with rows, a search without per-match content, and a payload that is
//! not a search at all (`None`).

pub mod Fn;
pub mod Payload;
pub mod Rows;
pub mod Total;
