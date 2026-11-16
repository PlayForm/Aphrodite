//! Response body of `POST /ccr/create`: the resulting hash and the reduction.
//!
//! The caller uses `hash` to build markers, so it must be the hash the store
//! actually recorded - not the key the caller asked for.

use serde::Serialize;

/// Response for `POST /ccr/create`, reporting the resulting hash and the
/// size reduction achieved.
#[derive(Debug, Serialize)]
pub struct Struct {
	pub hash:String,
	/// The reduction achieved, emitted as `token_savings_ratio` - the key is
	/// pinned because a rename must not change the wire contract. `1.0` when the
	/// original size is zero.
	#[serde(rename = "token_savings_ratio")]
	pub Ratio:f64,
	#[serde(rename = "original_size")]
	pub OriginalSize:usize,
	#[serde(rename = "compressed_size")]
	pub CompressedSize:usize,
	#[serde(rename = "marker_size")]
	pub MarkerSize:usize,
}
