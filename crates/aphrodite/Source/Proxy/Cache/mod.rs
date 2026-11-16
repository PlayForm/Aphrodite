//! The LLM response cache: request identity, freshness, and the lookup verdict.
//!
//! Three separate questions with three separate answers, because conflating
//! them is how a hit and a bypass came to look the same. `Identity` is what makes
//! two requests the same cacheable request - a digest to index by and the
//! canonical bytes that decide equality - and the LRU is keyed by it, so a hit is
//! equality rather than a digest match. `Freshness` classifies an entry against a
//! supplied clock and touches nothing. `Lookup` is the only place the LRU is
//! mutated: `peek` to classify, `get` to promote, `pop` to evict, so the eviction
//! order reflects what was actually served.

pub mod Canonical;
pub mod Entry;
pub mod Freshness;
pub mod Get;
pub mod Hash;
pub mod Identity;
pub mod Lookup;
pub mod Marker;
