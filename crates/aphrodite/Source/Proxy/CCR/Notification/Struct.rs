//! Webhook payload POSTed to `NotifyURL` when a CCR entry is created.
//!
//! Lets an external subscriber (Hermes) track store growth without polling.
//! Field names are the wire contract; adding a field is compatible, renaming
//! one is not.

use serde::Serialize;

/// Webhook payload POSTed to `NotifyURL` when a new CCR entry is created,
/// so external subscribers (e.g. Hermes) can track store growth without
/// polling.
#[derive(Debug, Serialize)]
pub struct Struct {
	#[serde(rename = "event")]
	pub Event:String,
	pub hash:String,
	#[serde(rename = "created_at")]
	pub CreatedAt:u64,
	#[serde(rename = "ttl")]
	pub Ttl:u64,
	#[serde(rename = "tags")]
	pub Tags:Vec<String>,
}

// ── Build state ─────────────────────────────────────────────────────
