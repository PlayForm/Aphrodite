//! Terminal path: exit-code typing, chain splitting, auto-expand.

use super::StoreNested;
use crate::{Hook::TransformTerminalOutput, State::AphroditeState};

#[test]
fn TestTerminalExitCode() {
	let mut s = AphroditeState::default();
	s.TerminalThreshold = 0;
	let r = TransformTerminalOutput(&mut s, "error: broke\nexit code: 1\n");
	assert_eq!(r["type"], "terminal");
}

#[test]
fn TestAutoExpandTerminalReturnsExpandedContent() {
	let mut s = AphroditeState::default();
	s.AutoExpand = true;
	s.AutoExpandLimit = 102400;
	s.TerminalThreshold = 0;
	let Nested = "terminal nested content";
	let NestedHash = StoreNested(&mut s, Nested);
	let Content = format!("line 1\n<<<CCR:{NestedHash}|text|{}>>>\nline 3", Nested.len());
	let r = TransformTerminalOutput(&mut s, &Content);
	assert_eq!(r["compressed"], true);
	assert_eq!(r["auto_expanded"], true);
	assert_eq!(
		r["expanded"],
		format!("line 1\n{Nested}\nline 3"),
		"expanded must resolve the nested marker inline"
	);
}

#[test]
fn TestAutoExpandTerminalOverLimitEmitsMarker() {
	let mut s = AphroditeState::default();
	s.AutoExpand = true;
	s.AutoExpandLimit = 4;
	s.TerminalThreshold = 0;
	let Content = "way over the four byte limit";
	let r = TransformTerminalOutput(&mut s, Content);
	assert_eq!(r["compressed"], true);
	assert!(r.get("auto_expanded").is_none(), "over-limit must emit the marker as today");
	assert!(r["marker"].as_str().unwrap().starts_with("<<<CCR:"));
	let hash = r["hash"].as_str().unwrap();
	assert_eq!(crate::Resolve::Expand::Fn(&mut s, hash), Some(Content.to_string()));
}

#[test]
fn TestAutoExpandDisabledTerminalPayloadIsByteIdentical() {
	let mut s = AphroditeState::default();
	s.TerminalThreshold = 0;
	let Content = "cargo build\n   Compiling aphrodite v0.1.0\nexit code: 0\n";
	let r = TransformTerminalOutput(&mut s, Content);
	assert_eq!(r["compressed"], true);
	assert!(r.get("auto_expanded").is_none());
	assert!(r.get("expanded").is_none());
	let hash = r["hash"].as_str().unwrap();
	let ExpectMarker = format!(
		"<<<CCR:{}|{}|{}>>>\n{}",
		hash,
		r["type"].as_str().unwrap(),
		Content.len(),
		r["preview"].as_str().unwrap()
	);
	assert_eq!(r["marker"], ExpectMarker);
	assert_eq!(crate::Resolve::Expand::Fn(&mut s, hash), Some(Content.to_string()));
}

// ── Gate interplay: auto-expand within the limit wins over chain-split
// and the below-threshold pass-through, so no marker can leak. ──

#[test]
fn TestAutoExpandBypassesChainSplitAndThreshold() {
	let mut s = AphroditeState::default();
	s.AutoExpand = true;
	s.AutoExpandLimit = 102400;
	s.ChainSplitEnabled = true;
	s.TerminalThreshold = 100000; // content below the threshold
	let Content = format!(
		"seg a output\n{}\nseg b output\n{}\nseg c output\n",
		crate::ChainSplit::SegMarker,
		crate::ChainSplit::SegMarker
	);
	let r = TransformTerminalOutput(&mut s, &Content);
	assert_eq!(
		r["auto_expanded"], true,
		"auto-expand must win over chain-split within the limit"
	);
	assert!(r.get("chain_split").is_none(), "no chain-split payload under auto-expand");
	assert_eq!(
		r["expanded"], Content,
		"expanded content must match the original (no nested markers)"
	);
}

#[test]
fn TestChainSplitUnaffectedWhenAutoExpandOff() {
	let mut s = AphroditeState::default(); // auto_expand = false
	s.ChainSplitEnabled = true;
	s.TerminalThreshold = 0;
	let Content = format!(
		"seg a output\n{}\nseg b output\n{}\nseg c output\n",
		crate::ChainSplit::SegMarker,
		crate::ChainSplit::SegMarker
	);
	let r = TransformTerminalOutput(&mut s, &Content);
	assert_eq!(r["chain_split"], true, "chain-split must keep working when auto-expand is off");
}
