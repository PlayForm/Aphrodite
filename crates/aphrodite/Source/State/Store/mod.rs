//! The inline content store - the session's retrieval source of truth.
//!
//! Everything an agent can later retrieve lives here, keyed by hash. Two
//! bounds apply at once and both must be enforced by the same eviction pass:
//!
//! - **Entry count** (`Entry::Const::Cap`) - the historical cap.
//! - **Bytes** (`Const::Budget`) - added by report 05 F11, because
//!   the entry cap alone let 500 entries of up to 16MB each reach multi-GB.
//!
//! The ordering inside `Put.rs` is the invariant that keeps both honest: the
//! running byte total is updated *before* the eviction pass, and the LRU cache
//! is sized one larger than the count cap so it can never self-evict an entry
//! that the pass would have accounted for. Get that backwards and the byte
//! total drifts from the map, which shows up as an entry that is present but
//! unretrievable, or a total that never comes back under budget.
//!
//! - `Struct.rs` - the store's fields: the map, its LRU order, both bounds.
//! - `Default.rs` - the seed: an empty map, its order, both bounds.
//! - `Set.rs` - the byte-budget setter.
//! - `Const.rs` - the default byte budget.
//! - `Entry/` - the entry-count cap.
//! - `Evict.rs` - the single eviction pass both bounds share.
//! - `Put.rs` - insert/overwrite with accounting.
//! - `Get.rs` - lookup with LRU promotion.
//! - `Contains.rs` - non-promoting membership test.

pub mod Const;
pub mod Contains;
pub mod Default;
pub mod Entry;
pub mod Evict;
pub mod Get;
pub mod Put;
pub mod Set;
pub mod Struct;
