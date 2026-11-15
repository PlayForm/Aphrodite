//! Live-container retrieval: the stored envelope and its reassembly.
//!
//! A live container is not a second store - it is the inline store's naming
//! convention read back (`Hook::Live::Key::Fn::live_meta_key` for the envelope,
//! `Hook::Live::Key::Fn::live_chunk_key` per chunk), so `Container/` is the envelope's shape
//! and `Lookup/` is the one function that reassembles content from it.

pub mod Container;
pub mod Lookup;
