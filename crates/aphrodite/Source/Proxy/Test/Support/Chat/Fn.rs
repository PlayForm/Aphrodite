//! A minimal Chat Completions body wrapping a piece of content.

pub(crate) fn ChatCompletionBody(Content:&str) -> Vec<u8> {
	serde_json::json!({
		"choices": [{
			"message": {"role": "assistant", "content": Content}
		}]
	})
	.to_string()
	.into_bytes()
}
