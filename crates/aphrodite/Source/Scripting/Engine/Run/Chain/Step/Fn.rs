//! Run one script against the chain.

use rhai::Engine as Rhai;

use crate::Scripting::{
	Engine::Run::{Chain::Struct as Chain, Scope::Fn::Fn as SeedScope},
	Payload::Struct::Struct as HookPayload,
	Script::{Declares, Struct::Struct as Script},
};

/// Evaluate one script's hook and return the chain it produced.
///
/// A script that cannot be compiled, or that does not declare the hook, returns the
/// chain UNCHANGED - both are "this script is not for this call", not failures. The
/// compile cannot normally fail (the source was validated at reload), so a failure here
/// means the file was injected directly.
///
/// A script that throws is warned and skipped: its input passes through unchanged. A
/// failing script must never crash the proxy, which is why the error arm returns the
/// chain rather than a `Result` the caller would have to decide what to do with.
///
/// The whole step consumes the chain and produces the next one, so the fold in
/// `Run::Fn::Fn` is the loop and this leaf is one iteration of it.
pub(crate) fn Fn(Chain:Chain, Script:&Script, Hook:&str, Payload:&HookPayload) -> Chain {
	// Compiled per call - no AST storage, avoids Send issues.
	let Engine = Rhai::new();
	let Ok(Ast) = Engine.compile(&Script.Source) else {
		return Chain;
	};

	let Some(Hook) = Declares::Fn::Fn(&Ast, Hook) else {
		return Chain;
	};

	let mut Scope = SeedScope(Payload, Hook, &Chain.Content);

	match Engine.call_fn::<String>(&mut Scope, &Ast, Hook, ()) {
		Ok(Transformed) => Chain { Content:Transformed, Touched:true },
		Err(Error) => {
			tracing::warn!(
				Script = %Script.path.display(),
				Hook,
				error = %Error,
				"script hook failed - degraded to previous value"
			);

			Chain
		},
	}
}
