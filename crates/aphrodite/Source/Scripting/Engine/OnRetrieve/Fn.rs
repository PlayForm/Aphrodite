//! `OnRetrieve`: the retrieval hook surface.

use crate::Scripting::{Engine::Struct::Struct, Payload::Struct::Struct as HookPayload};

impl Struct {
	/// Pre-Rust compatibility surface: run the `OnRetrieve` hook over
	/// content. Returns the transformed content, or the input unchanged when
	/// no script modified it.
	///
	/// `Preview` is not seeded here: on this path the preview is derived from
	/// the content the caller is about to return, so a script that rewrote
	/// `Preview` would be rewriting a value nobody reads.
	pub fn OnRetrieve(&self, hash:&str, Content:&str, Ct:&str) -> String {
		let Payload = HookPayload {
			Content:Some(Content.to_string()),
			ContentType:Some(Ct.to_string()),
			hash:Some(hash.to_string()),
			..Default::default()
		};
		self.run("on_retrieve", &Payload).unwrap_or_else(|| Content.to_string())
	}
}
