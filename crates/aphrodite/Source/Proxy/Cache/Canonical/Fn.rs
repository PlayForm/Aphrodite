//! Canonical bytes for a parsed request: the cache identity's equality evidence.
//!
//! A digest answers "which bucket", never "is this the same request". The cache
//! identity therefore carries the canonical bytes as well as their digest, and
//! this leaf produces the bytes.
//!
//! Canonical means two things here. It covers the WHOLE parsed body plus the
//! credential - not a hand-written field list, which is how a request differing
//! only in `max_tokens`, `stop` or `seed` came to be served another request's
//! answer, and how every field added to the Chat Completions API afterwards
//! silently aliased an existing entry. And it is order-insensitive: object
//! members are emitted in sorted key order, so two clients that send the same
//! request with different key order share one identity instead of missing each
//! other's entries. `serde_json` is built with `preserve_order` in this
//! workspace, so a plain `to_vec` would keep the client's order and make the
//! identity a function of the request text rather than of its contract.

use serde_json::Value;

/// The canonical byte form of `Body` under `APIKey`.
pub(crate) fn Fn(Body:&Value, APIKey:&str) -> Vec<u8> {
	let mut Out:Vec<u8> = Vec::new();

	// The credential is a length-prefixed field, not a bare prefix: without a
	// separator a key ending in `=` and a body starting with `{` could spell the
	// same bytes as a different (key, body) pair.
	Out.extend_from_slice(format!("key:{}:", APIKey.len()).as_bytes());
	Out.extend_from_slice(APIKey.as_bytes());
	Out.push(b'\n');
	WriteValue(Body, &mut Out);

	Out
}

/// Emit `Value` with every object's members in sorted key order.
fn WriteValue(Value:&Value, Out:&mut Vec<u8>) {
	match Value {
		serde_json::Value::Object(Members) => {
			let mut Keys:Vec<&String> = Members.keys().collect();

			Keys.sort_unstable();
			Out.push(b'{');

			for (Index, Key) in Keys.into_iter().enumerate() {
				if Index > 0 {
					Out.push(b',');
				}

				// `to_writer` is the only escaper in this walk: a key is JSON
				// text, and re-escaping it by hand is how a canonical form stops
				// being canonical.
				let _ = serde_json::to_writer(&mut *Out, Key);
				Out.push(b':');
				WriteValue(&Members[Key.as_str()], Out);
			}

			Out.push(b'}');
		},
		serde_json::Value::Array(Items) => {
			Out.push(b'[');

			for (Index, Item) in Items.iter().enumerate() {
				if Index > 0 {
					Out.push(b',');
				}

				WriteValue(Item, Out);
			}

			Out.push(b']');
		},
		Scalar => {
			let _ = serde_json::to_writer(Out, Scalar);
		},
	}
}
