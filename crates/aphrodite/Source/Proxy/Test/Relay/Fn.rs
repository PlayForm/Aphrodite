//! Tool relay: the wire shapes of request and response, and
//! `ExecuteToolRelay`'s closed verb set.

use crate::Proxy::{
	Relay::{Execute::Fn::Fn as ExecuteToolRelay, Response::Struct::Struct as ToolRelayResponse},
	State::CCR::Get::Fn::Fn as CCRGet,
	Test::Support::State::Fn::TestStateWithCCR,
};

#[test]
fn TestToolRelayResponseSyncSerdeShape() {
	let Response = ToolRelayResponse {
		Success:true,
		Result:Some(serde_json::json!({"found": true})),
		error:None,
		AsyncCall:false,
	};
	let v = serde_json::to_value(&Response).unwrap();
	assert_eq!(v["success"], true);
	assert_eq!(v["async_call"], false);
	assert_eq!(v["result"]["found"], true);
	assert!(v["error"].is_null());
}

#[test]
fn TestToolRelayResponseAsyncSerdeShape() {
	let Response = ToolRelayResponse { Success:true, Result:None, error:None, AsyncCall:true };
	let v = serde_json::to_value(&Response).unwrap();
	assert_eq!(v["async_call"], true);
	assert!(v["result"].is_null());
}

// ── T5: execute_tool_relay ───────────────────────────────────
#[test]
fn TestExecuteToolRelayRetrieveMissingHashParameter() {
	let State = TestStateWithCCR();
	let Result = tokio::runtime::Runtime::new().unwrap().block_on(ExecuteToolRelay(
		&State,
		"aphrodite_retrieve",
		&serde_json::json!({}),
	));
	let Error = format!("{:#}", Result.expect_err("a retrieve with no hash argument must be an error"));
	assert!(Error.contains("missing hash"), "unexpected error: {Error}");
}

#[test]
fn TestExecuteToolRelayUnknownToolIsError() {
	let State = TestStateWithCCR();
	let Result = tokio::runtime::Runtime::new().unwrap().block_on(ExecuteToolRelay(
		&State,
		"not_a_real_tool",
		&serde_json::json!({}),
	));
	assert!(Result.is_err());
}

#[test]
fn TestExecuteToolRelayCompressSmallContentStoresInlineAndReturnsMarker() {
	let State = TestStateWithCCR();
	let Content = "tiny"; // well under INLINE_CCR_THRESHOLD (256B)
	let Result = tokio::runtime::Runtime::new().unwrap().block_on(ExecuteToolRelay(
		&State,
		"aphrodite_compress",
		&serde_json::json!({"content": Content}),
	));
	let v = Result.expect("compress must succeed");
	assert!(v["compressed"].as_str().unwrap().contains("<<<CCR:"));
	assert_eq!(v["original_size"], Content.len());
}

// ── T7 (F5, report 06): tiny relay-compress content must also land in
// the durable backend, not only the 1024-entry inline LRU - otherwise a
// busy session evicts the "durable-looking" marker's only copy within
// minutes. Verifies the durable store has the content directly,
// independent of the inline cache. ──
#[test]
fn TestExecuteToolRelayCompressSmallContentAlsoStoresDurably() {
	let State = TestStateWithCCR();
	let Content = "tiny"; // well under INLINE_CCR_THRESHOLD (256B)
	let Rt = tokio::runtime::Runtime::new().unwrap();
	let Result = Rt.block_on(ExecuteToolRelay(
		&State,
		"aphrodite_compress",
		&serde_json::json!({"content": Content}),
	));
	let v = Result.expect("compress must succeed");
	let hash = v["hash"].as_str().unwrap().to_string();

	// Simulate the inline entry having been evicted (busy session /
	// restart) by going straight to the durable backend.
	let ccr = State.ccr.as_ref().unwrap();
	let Durable = Rt.block_on(CCRGet(ccr, &hash));
	assert_eq!(
		Durable.as_deref(),
		Some(Content),
		"tiny content must also be durable, not inline-only"
	);
}

#[test]
fn TestExecuteToolRelayRetrieveFindsInlineEntry() {
	let State = TestStateWithCCR();
	let Content = "tiny";
	let Compressed = tokio::runtime::Runtime::new()
		.unwrap()
		.block_on(ExecuteToolRelay(
			&State,
			"aphrodite_compress",
			&serde_json::json!({"content": Content}),
		))
		.unwrap();
	let hash = Compressed["hash"].as_str().unwrap().to_string();

	let Retrieved = tokio::runtime::Runtime::new()
		.unwrap()
		.block_on(ExecuteToolRelay(
			&State,
			"aphrodite_retrieve",
			&serde_json::json!({"hash": hash}),
		))
		.unwrap();
	assert_eq!(Retrieved["found"], true);
	assert_eq!(Retrieved["content"], Content);
}

// ── T5 (F3): execute_tool_relay's "aphrodite_retrieve" arm must
// normalize the hash argument the same way `ResolveOne` already does -
// strip a `|type|size` marker-body suffix an LLM might echo back, and
// trim surrounding whitespace.
#[test]
fn TestExecuteToolRelayRetrieveNormalizesPipeSuffixedAndWhitespaceHash() {
	let State = TestStateWithCCR();
	let Content = "tiny";
	let Rt = tokio::runtime::Runtime::new().unwrap();
	let Compressed = Rt
		.block_on(ExecuteToolRelay(
			&State,
			"aphrodite_compress",
			&serde_json::json!({"content": Content}),
		))
		.unwrap();
	let hash = Compressed["hash"].as_str().unwrap().to_string();

	[hash.clone(), format!("{hash}|tool|1024"), format!("  {hash}  ")]
		.into_iter()
		.for_each(|HashArgument| {
			let Retrieved = Rt
				.block_on(ExecuteToolRelay(
					&State,
					"aphrodite_retrieve",
					&serde_json::json!({"hash": HashArgument}),
				))
				.unwrap();
			assert_eq!(
				Retrieved["found"], true,
				"hash arg {HashArgument:?} must resolve: {Retrieved:?}"
			);
			assert_eq!(Retrieved["content"], Content);
		});
}

// ── T15 (F2): regression tests for the historical corpus examples ────
// These exercise the real Rust code (unlike Maintain/examples/*.py,
// which re-implement the buggy/fixed logic in Python and can never
// catch a Rust regression - see Maintain/examples/README.md).
