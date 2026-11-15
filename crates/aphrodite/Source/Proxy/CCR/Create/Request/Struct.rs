//! Inbound body of `POST /ccr/create`.
//!
//! `Content` is stored under an optional caller-supplied `Key`, defaulting to
//! the content hash. Field names are the wire contract.

use serde::Deserialize;

/// Inbound request body for `POST /ccr/create`: store `Content` under an
/// optional caller-supplied `Key` (defaults to the content hash).
#[derive(Debug, Deserialize)]
pub struct Struct {
	pub Content:String,
	pub Key:Option<String>,
	pub TtlSeconds:Option<u64>,
	pub Tags:Option<Vec<String>>,
}
