//! A minimal Chat Completions body wrapping a piece of content.

/// A Chat Completions response body whose single choice carries `Content` -
/// the shape `Compress::Chat` parses, as bytes.
pub(crate) fn Payload(Content:&str) -> Vec<u8> {
	serde_json::json!({
		"choices": [{
			"message": {"role": "assistant", "content": Content}
		}]
	})
	.to_string()
	.into_bytes()
}
