//! The LLM response cache: key derivation, freshness, and the lookup verdict.
//!
//! Three separate questions with three separate answers, because conflating
//! them is how a hit and a bypass came to look the same. `Key` derives the
//! digest - a cache-format decision, not a style one. `Freshness` classifies an
//! entry against a supplied clock and touches nothing. `Lookup` is the only
//! place the LRU is mutated: `peek` to classify, `get` to promote, `pop` to
//! evict, so the eviction order reflects what was actually served.

pub mod Entry;
pub mod Freshness;
pub mod Get;
pub mod Hash;
pub mod Key;
pub mod Lookup;
pub mod Marker;
