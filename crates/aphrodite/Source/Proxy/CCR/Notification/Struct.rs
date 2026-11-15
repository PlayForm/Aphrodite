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
	pub Event:String,
	pub hash:String,
	pub CreatedAt:u64,
	pub Ttl:u64,
	pub Tags:Vec<String>,
}

// ── Build state ─────────────────────────────────────────────────────
