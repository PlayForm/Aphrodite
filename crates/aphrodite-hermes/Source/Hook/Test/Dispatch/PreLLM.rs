//! The `pre_llm_call` arm: directives must reach the model on the Hermes path.

use std::ffi::{CStr, CString};

use crate::{
	ABI::{CallHook::aphrodite_hermes_call_hook, FreeString::aphrodite_hermes_free_string},
	Session::With::Fn::with_shared,
};

// ── 01-F3: the bridge's `pre_llm_call` arm - the only one Hermes calls
// in production - must actually inject active directive text, not just
// the catalog summary. Previously it called `Session::catalog_summary`
// directly and never touched `Directive::build_directive_context` at
// all, so the "wire directives into pre_llm_call" feature was dead
// end-to-end despite `Hook::pre_llm_call` (core, unreachable from this
// path) already building that context correctly. ──
#[test]
fn test_call_hook_pre_llm_call_injects_active_directive_context() {
	let _g = crate::Session::Guard::Fn::test_guard();
	aphrodite_hermes_call_hook(
		CString::new("session_start").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);
	with_shared(|state| {
		state.Directives.insert(
			"focus".into(),
			aphrodite::Directive::Model::Struct { name:"focus".into(), Content:"stay concise, 1-2 tools/turn".into() },
		);
		state.ActiveDirectives = vec!["focus".into()];
	});

	let hook_ptr = aphrodite_hermes_call_hook(
		CString::new("pre_llm_call").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);
	let result = unsafe { CStr::from_ptr(hook_ptr) }.to_string_lossy().into_owned();
	aphrodite_hermes_free_string(hook_ptr);

	let v:serde_json::Value = serde_json::from_str(&result).unwrap();
	let context = v["context"].as_str().unwrap_or_default();
	assert!(
		context.contains("[directives: focus]"),
		"context missing directive marker: {context}"
	);
	assert!(context.contains("stay concise"), "context missing directive body: {context}");

	// `active_directives` is process-global and not reset by
	// session_start (deliberately) - clean up so this test doesn't leak
	// an active directive into whichever test runs next.
	with_shared(|state| state.ActiveDirectives.clear());
}

// ── 05-T1 (P1): the end-to-end proof that directives inject through the
// single `Flow::build_turn_context` assembler on the production Hermes
// path. Activate a directive, call the `pre_llm_call` hook, and assert the
// `[directives:` marker appears in the returned `context`. This is the
// choke point that makes the G1 bug class (directives dead on the Hermes
// path) unrepresentable. ──
#[test]
fn test_call_hook_pre_llm_includes_directives() {
	let _g = crate::Session::Guard::Fn::test_guard();
	aphrodite_hermes_call_hook(
		CString::new("session_start").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);
	with_shared(|state| {
		state.Directives.insert(
			"focus".into(),
			aphrodite::Directive::Model::Struct { name:"focus".into(), Content:"stay targeted, 1-2 tools".into() },
		);
		state.ActiveDirectives = vec!["focus".into()];
	});

	let hook_ptr = aphrodite_hermes_call_hook(
		CString::new("pre_llm_call").unwrap().as_ptr(),
		CString::new("{}").unwrap().as_ptr(),
	);
	let result = unsafe { CStr::from_ptr(hook_ptr) }.to_string_lossy().into_owned();
	aphrodite_hermes_free_string(hook_ptr);

	let v:serde_json::Value = serde_json::from_str(&result).unwrap();
	let context = v["context"].as_str().unwrap_or_default();
	assert!(
		context.contains("[directives: focus]"),
		"assembler must inject directives on the Hermes path: {context}"
	);
	assert!(context.contains("stay targeted"), "directive body missing: {context}");

	with_shared(|state| state.ActiveDirectives.clear());
}
