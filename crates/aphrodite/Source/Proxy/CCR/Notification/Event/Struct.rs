//! Webhook payload POSTed to `NotifyURL` when a CCR entry is created.
//!
//! Lets an external subscriber (Hermes) track store growth without polling.
//! Field names are the wire contract; adding a field is compatible, renaming one
//! is not.

use serde::Serialize;

/// Webhook payload POSTed to `NotifyURL` when a new CCR entry is created.
#[derive(Debug, Serialize)]
pub(crate) struct Struct {
	#[serde(rename = "event")]
	pub(crate) Event:String,
	pub(crate) hash:String,
	#[serde(rename = "created_at")]
	pub(crate) CreatedAt:u64,
	#[serde(rename = "ttl")]
	pub(crate) Ttl:u64,
	#[serde(rename = "tags")]
	pub(crate) Tags:Vec<String>,
}
