//! `Run`: evaluate every script's hook function.

use crate::Scripting::{
	Engine::{
		Run::Chain::{Finish, Step, Struct as Chain},
		Struct::Struct,
	},
	Payload::Struct::Struct as HookPayload,
};

impl Struct {
	/// Evaluate every loaded script's `fn <hook>()` against `Payload`, chaining each
	/// script's output into the next script's `Content`.
	///
	/// Returns `Some(transformed)` when at least one script modified the payload, `None`
	/// when nothing changed. A script that errors (throws, returns the wrong type,
	/// panics) is warned and skipped - its input passes through unchanged; a script that
	/// does not define the hook function is skipped silently (a missing function is not
	/// an error). All lock/compile/call sites degrade; this method never panics.
	///
	/// The fold IS the chain: each step consumes the accumulator and returns the next
	/// one, so "the content the next script sees" and "whether anything ran" cannot fall
	/// out of step, and there is no early `return` inside a `for_each` for a reader to
	/// trace. `Finish` is the one place the accumulator becomes the hook's answer.
	pub fn Run(&self, Hook:&str, Payload:&HookPayload) -> Option<String> {
		self.CheckReload();

		let Scripts = self.Scripts.lock().unwrap_or_else(|Error| Error.into_inner());

		let Chained = Scripts.iter().fold(
			Chain { Content:Payload.Content.clone().unwrap_or_default(), Touched:false },
			|Chained, Script| Step::Fn::Fn(Chained, Script, Hook, Payload),
		);

		Finish::Fn::Fn(Chained)
	}
}
