//! Tool-result path: bypass gates, the payload shape, auto-expand.

use super::StoreNested;
use crate::{
	Hook::{Hash::Fn::Fn as ComputeHash, ToolResult::Entry::Fn::TransformToolResult},
	State::Struct::AphroditeState,
};

#[test]
fn TestEssentialToolSkip() {
	let mut s = AphroditeState::default();
	let r = TransformToolResult(&mut s, "some content", "skill_view");
	assert_eq!(r["compressed"], false);
	assert_eq!(r["reason"], "essential_tool");
}

// ── T9 (F10): `read_file` is both an essential tool (never compressed -
// the agent needs the raw content) AND a tracked file tool
// (`state.file_tools`); both must hold at once. Previously the essential-
// tool early return ran before file-reference tracking, so the single
// most common file reference (reading a file) was silently never
// recorded in `ReferencedFiles`.
#[test]
fn TestEssentialToolReadFileStillRecordsFileReference() {
	let mut s = AphroditeState::default();
	let r = TransformToolResult(&mut s, "/tmp/some/file.rs\nfn main() {}\n", "read_file");
	assert_eq!(r["compressed"], false, "read_file must never be compressed");
	assert_eq!(r["reason"], "essential_tool");
	assert_eq!(
		s.ReferencedFiles.len(),
		1,
		"read_file must still be tracked as a file reference"
	);
	assert_eq!(s.ReferencedFiles[0].0, "/tmp/some/file.rs");
}

// ── T9 (F10): `search_files` reaches the file-reference tracker (it's
// listed in `state.file_tools`) but `ExtractFilePath` had no arm for
// it, so search results were never recorded despite being tracked tools.
#[test]
fn TestSearchFilesRecordsFileReference() {
	let mut s = AphroditeState::default();
	s.ToolThreshold = 0; // always compress, to also exercise the non-essential path
	let Content = "/tmp/some/file.rs:42:    let x = 1;\n/tmp/other/file.rs:7:    let y = 2;\n";
	let _ = TransformToolResult(&mut s, Content, "search_files");
	assert_eq!(s.ReferencedFiles.len(), 1);
	assert_eq!(s.ReferencedFiles[0].0, "/tmp/some/file.rs");
}

#[test]
fn TestEmptySkip() {
	let mut s = AphroditeState::default();
	let r = TransformToolResult(&mut s, "", "terminal");
	assert_eq!(r["compressed"], false);
}

#[test]
fn TestBelowThreshold() {
	let mut s = AphroditeState::default();
	s.ToolThreshold = 10000;
	let r = TransformToolResult(&mut s, "short", "terminal");
	assert_eq!(r["compressed"], false);
}

#[test]
fn TestTransformSuccess() {
	let mut s = AphroditeState::default();
	s.ToolThreshold = 0; // always compress
	let Content = "fn main() {\n    println!(\"hello world\");\n}\n";
	let r = TransformToolResult(&mut s, Content, "terminal");
	assert_eq!(r["compressed"], true);
	let hash = r["hash"].as_str().unwrap();
	assert!(hash.len() >= 40);
	// The round-trip is the actual promise here: the marker this hook hands
	// back to the LLM must resolve, via aphrodite_retrieve, back to the
	// exact original content - not merely produce a hash-shaped string.
	let Resolved = crate::Resolve::Expand::Fn(&mut s, hash);
	assert_eq!(Resolved, Some(Content.to_string()));
}

// ── Auto-expand (dylib-side consumer of `[compression] auto_expand`): when
// enabled and content fits `AutoExpandLimit`, the hook returns the RESOLVED
// content inline (no `<<<CCR:...>>>` marker reaches the model); over the limit
// or disabled, today's marker payload is byte-identical. ──

#[test]
fn TestAutoExpandReturnsExpandedContent() {
	let mut s = AphroditeState::default();
	s.AutoExpand = true;
	s.AutoExpandLimit = 102400;
	s.ToolThreshold = 0;
	// A nested entry whose marker text is embedded in the tool output:
	// the resolved content must differ from the raw marker text, proving
	// the hook actually resolves instead of just passing through.
	let Nested = "nested payload\nsecond line\n";
	let NestedHash = StoreNested(&mut s, Nested);
	let Content = format!("<<<CCR:{NestedHash}|text|{}>>>", Nested.len());
	let r = TransformToolResult(&mut s, &Content, "terminal");
	assert_eq!(r["compressed"], true);
	assert_eq!(r["auto_expanded"], true, "auto-expand must flag the payload");
	assert_eq!(r["expanded"], Nested, "expanded must be the resolved nested content");
	// The marker is still recorded and resolvable (retrieval stays
	// lossless). The stored content was a single wrapper marker, so
	// recursive expansion lands on the nested text itself.
	let ContentHash = r["hash"].as_str().unwrap().to_string();
	assert!(r["marker"].as_str().unwrap().starts_with("<<<CCR:"));
	assert_eq!(crate::Resolve::Expand::Fn(&mut s, &ContentHash), Some(Nested.to_string()));
	assert_eq!(crate::Resolve::Expand::Fn(&mut s, &NestedHash), Some(Nested.to_string()));
}

#[test]
fn TestAutoExpandOverLimitEmitsMarker() {
	let mut s = AphroditeState::default();
	s.AutoExpand = true;
	s.AutoExpandLimit = 8;
	s.ToolThreshold = 0;
	let Content = "this content is far longer than eight bytes";
	let r = TransformToolResult(&mut s, Content, "terminal");
	assert_eq!(r["compressed"], true);
	assert!(
		r.get("auto_expanded").is_none(),
		"over-limit content must keep today's marker payload (no auto-expand fields)"
	);
	assert!(r.get("expanded").is_none());
	assert!(r["marker"].as_str().unwrap().starts_with("<<<CCR:"));
	// The stored entry must still resolve to the exact original.
	let hash = r["hash"].as_str().unwrap();
	assert_eq!(crate::Resolve::Expand::Fn(&mut s, hash), Some(Content.to_string()));
}

// Default (auto_expand = false): the payload must be byte-identical to
// the pre-auto-expand shape - same field set, same marker grammar.
#[test]
fn TestAutoExpandDisabledPayloadIsByteIdentical() {
	let mut s = AphroditeState::default(); // auto_expand = false
	s.ToolThreshold = 0;
	let Content = "fn main() {\n    println!(\"hello\");\n}\n";
	let r = TransformToolResult(&mut s, Content, "terminal");
	assert_eq!(r["status"], "ok");
	assert_eq!(r["compressed"], true);
	assert!(r.get("auto_expanded").is_none());
	assert!(r.get("expanded").is_none());
	let hash = r["hash"].as_str().unwrap();
	assert_eq!(hash, &ComputeHash(Content));
	assert_eq!(r["size"], Content.len() as u64);
	let mut Keys:Vec<&str> = r.as_object().unwrap().keys().map(|k| k.as_str()).collect();
	let mut Expected = vec!["status", "compressed", "hash", "type", "size", "preview", "marker"];
	Keys.sort();
	Expected.sort();
	assert_eq!(Keys, Expected, "default path must keep the exact pre-auto-expand field set");
	let ExpectMarker = format!(
		"<<<CCR:{}|{}|{}>>>\n{}",
		hash,
		r["type"].as_str().unwrap(),
		Content.len(),
		r["preview"].as_str().unwrap()
	);
	assert_eq!(r["marker"], ExpectMarker, "default path must render the exact marker grammar");
	assert_eq!(crate::Resolve::Expand::Fn(&mut s, hash), Some(Content.to_string()));
}

#[test]
fn TestAutoExpandBypassesToolThresholdGate() {
	let mut s = AphroditeState::default();
	s.AutoExpand = true;
	s.AutoExpandLimit = 102400;
	s.ToolThreshold = 10000; // content below the threshold
	let Content = "tiny tool output";
	let r = TransformToolResult(&mut s, Content, "terminal");
	assert_eq!(
		r["auto_expanded"], true,
		"auto-expand must store+expand below-threshold content so no marker leaks"
	);
	assert_eq!(r["expanded"], Content);
}
