//! Inbound body of `POST /ccr/create`.
//!
//! `Content` is stored under an optional caller-supplied `Key`, defaulting to
//! the content hash. Field names are the wire contract.

use serde::Deserialize;

/// Inbound request body for `POST /ccr/create`: store `Content` under an
/// optional caller-supplied `Key` (defaults to the content hash).
#[derive(Debug, Deserialize)]
pub struct Struct {
	#[serde(rename = "content")]
	pub Content:String,
	#[serde(rename = "key")]
	pub Key:Option<String>,
	#[serde(rename = "ttl_seconds")]
	pub TtlSeconds:Option<u64>,
	#[serde(rename = "tags")]
	pub Tags:Option<Vec<String>>,
}
