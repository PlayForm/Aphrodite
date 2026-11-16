//! The `aphrodite_directive` bridge tool.
//!
//! 01-F3: the bridge tool (not the core C ABI export) must be dispatchable and
//! delegate to the same `Directive::handle_action` as everything else -
//! list/swap round-trip through the shared session state.

use crate::Tool::Dispatch::Fn::Fn as dispatch;

#[test]
fn test_aphrodite_directive_tool_list_and_swap() {
	let _g = crate::Session::Guard::Fn::test_guard();
	crate::Session::With::Fn::with_shared(|state| {
		// `active_directives` is process-global and NOT reset between
		// tests (deliberately - directives persist across a session
		// reset just like the inline store); start from a known-empty
		// state rather than assuming one.
		state.Directive.Active.clear();
		state.Directive.Loaded.insert(
			"focus".into(),
			aphrodite::Directive::Model::Struct { name:"focus".into(), Content:"stay focused".into() },
		);
	});

	let listed = dispatch("aphrodite_directive", "{}");
	assert_eq!(listed["available"], serde_json::json!(["focus"]));
	assert_eq!(listed["active"], serde_json::json!([]));

	let swapped = dispatch(
		"aphrodite_directive",
		&serde_json::json!({"action": "swap", "name": "focus"}).to_string(),
	);
	assert_eq!(swapped["swapped"], "focus");
	assert_eq!(swapped["active"], serde_json::json!(["focus"]));
}
