//! `/retrieve` endpoint - resolve CCR markers to original content.
//!
//! Retrieval is the other half of the compression contract: a marker is only
//! worth emitting if the bytes behind it can be handed back exactly. That is
//! why `Page/` never reconstructs a full document through `lines()`/`join()`
//! and why `Handle/` resolves the inline store before the CCR backend - the
//! returned body has to hash back to the marker's own hash.
//!
//! The tree follows the request's own path: `Request/` and `Response/` are the
//! wire shapes, `Live/` reassembles a live container (envelope in
//! `Container/`, reassembly in `Lookup/`), `Filter/` narrows by query,
//! `Page/` slices the window, `Progress/` is the live-container progress
//! carried back on the response, and `Handle/` is the one function that runs
//! them in order.
//!
//! This facade declares the tree and nothing else. A consumer names the leaf
//! (`Retrieve::Handle::Fn::Fn`), so the file that defines an item is the only
//! place it is defined and there is exactly one way to spell it.

pub mod Filter;
pub mod Handle;
pub mod Live;
pub mod Page;
pub mod Progress;
pub mod Request;
pub mod Response;

#[cfg(test)]
mod Test;
