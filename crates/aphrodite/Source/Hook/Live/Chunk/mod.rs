//! Chunking for live containers: the target policy and the iterator it cuts with.
//!
//! Two items, because they answer different questions: `Policy/` says what a
//! chunk IS - a target size, cut at or beyond it, never zero - and `Iter/`
//! produces the chunks without collecting them. The container enumerates, stores
//! and counts in one pass, so no descriptor vector is built and thrown away.

pub mod Const;
pub mod Iter;
pub mod Policy;
