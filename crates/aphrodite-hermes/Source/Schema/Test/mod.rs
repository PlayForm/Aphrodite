//! Unit tests for the tool schemas, split by the contract they defend.
//!
//! Three independent contracts, three areas: the catalogue's tool COUNT (what
//! `hermes plugins validate` admits), the SHAPE of each record (what the
//! sanitizers and `tool_describe` assume), and RESOLUTION (that the catalogue,
//! the lookup and the dispatch registry agree).

mod Manifest;
mod Resolve;
mod Shape;
