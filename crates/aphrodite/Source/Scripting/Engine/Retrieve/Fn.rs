//! `Retrieve`: the retrieval hook surface.

use crate::Scripting::{Engine::Struct::Struct, Payload::Struct::Struct as HookPayload};

impl Struct {
	/// Pre-Rust compatibility surface: run the `on_retrieve` hook over
	/// content. Returns the transformed content, or the input unchanged when
	/// no script modified it.
	///
	/// `Preview` is not seeded here: on this path the preview is derived from
	/// the content the caller is about to return, so a script that rewrote
	/// `Preview` would be rewriting a value nobody reads.
	pub fn Retrieve(&self, hash:&str, Content:&str, Ct:&str) -> String {
		let Payload = HookPayload {
			Content:Some(Content.to_string()),
			Type:Some(Ct.to_string()),
			Hash:Some(hash.to_string()),
			..Default::default()
		};
		self.Run("on_retrieve", &Payload).unwrap_or_else(|| Content.to_string())
	}
}
