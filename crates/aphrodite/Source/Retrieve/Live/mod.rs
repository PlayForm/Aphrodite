//! Live-container retrieval: the stored envelope, its reassembly and its progress.
//!
//! A live container is not a second store - it is the inline store's naming
//! convention read back (`Hook::Live::Key::Fn::LiveMetaKey` for the envelope,
//! `Hook::Live::Key::Fn::LiveChunkKey` per chunk), so `Container/` is the
//! envelope's shape, `Lookup/` reassembles content from it, and `Advance/`
//! reports and persists how much of it a retrieve has delivered.
//!
//! `Advance/` is separate from `Lookup/` because it is the only half that writes:
//! reassembly is a pure read of the chunks, while progress moves the envelope
//! forward and must degrade rather than fail the request.

pub mod Advance;
pub mod Container;
pub mod Lookup;
