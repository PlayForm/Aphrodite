//! The recent-marker catalog.
//!
//! Every marker the engine emits is recorded here so later turns can render a
//! catalog of what is retrievable. The ring is bounded and front-evicting, so
//! index 0 is always the oldest survivor - the catalog renderer and the delta
//! counters depend on that, not on insertion order.
//!
//! `Entry` is `serde::Serialize` because `AphroditeCatalog` hands it
//! straight to the model as JSON; its field names are therefore part of a wire
//! shape and stay `snake_case`.
//!
//! - `Struct.rs` - the catalog's fields - the ring and the delta counter - and
//!   their seed.
//! - `Entry/` - one catalog entry (the item).
//! - `Const.rs` - the ring's cap.

pub mod Const;
pub mod Entry;
pub mod Record;
pub mod Struct;
