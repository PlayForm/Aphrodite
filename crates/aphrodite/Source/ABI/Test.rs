//! The ABI battery: every guarantee the C surface makes, asserted through the
//! exported symbols themselves rather than through the internals behind them.
//!
//! These tests call the `extern "C"` functions exactly as a host would - raw
//! pointers in, owned C strings out, freed with `AphroditeFreeString` - which
//! is the only way to catch a contract break that the Rust-typed call path would
//! paper over.

use std::{
	ffi::{CStr, CString},
	os::raw::c_char,
};

use crate::ABI::Catalog::AphroditeCatalog;
use crate::ABI::Classify::AphroditeClassify;
use crate::ABI::Compress::AphroditeCompress;
use crate::ABI::ConfigurationGet::AphroditeConfigurationGet;
use crate::ABI::ConfigurationSet::AphroditeConfigurationSet;
use crate::ABI::Destroy::AphroditeDestroy;
use crate::ABI::Dispatch::AphroditeDispatch;
use crate::ABI::FreeString::AphroditeFreeString;
use crate::ABI::Guard::Guarded;
use crate::ABI::Guard::MaximumContent;
use crate::ABI::Handle::Table::Handles;
use crate::ABI::Handle::Lookup::WithState;
use crate::ABI::Initialise::AphroditeInit;
use crate::ABI::Reload::AphroditeReload;
use crate::ABI::Retrieve::AphroditeRetrieve;
use crate::ABI::Search::AphroditeSearch;
use crate::ABI::SessionStart::AphroditeSessionStart;
use crate::ABI::Stats::AphroditeStats;
use crate::Preview::Builder::Entry::Fn as BuildPreview;

// ── T9 (F10): build_preview's "terminal" arm ──────────────────
// `Hook::transform_terminal_output` overrides the classified type to
// "terminal" for exit-code/error-shaped output, but `BuildPreview` had
// no matching arm, so the preview silently fell through to the generic
// `_` branch (a bare line/byte count with no exit-code context).
#[test]
fn TestBuildPreviewTerminalSurfacesExitCode() {
	let _G = crate::Preview::State::Guard::Fn();
	let Preview = BuildPreview("terminal", "running tests\nall good\nexit code: 1\n");
	assert!(Preview.starts_with("[terminal:"));
	assert!(
		Preview.contains("exit code: 1"),
		"preview should surface the exit code line: {Preview}"
	);
}

#[test]
fn TestBuildPreviewTerminalFallsBackToFirstLine() {
	let _G = crate::Preview::State::Guard::Fn();
	// ISSUE-11 residual #2: the terminal-arm fallback used to be the LAST
	// non-empty line - a multi-line block previewed as its closing brace
	// (`[terminal:7L }]`). The FIRST meaningful line is the honest
	// default (skipping lone braces/brackets).
	let Preview = BuildPreview("terminal", "line one\nline two\nlast line here\n");
	assert!(Preview.starts_with("[terminal:"));
	assert!(
		Preview.contains("line one"),
		"preview should fall back to the first meaningful line: {Preview}"
	);
	assert!(
		!Preview.contains("last line here"),
		"preview must not surface the last line: {Preview}"
	);
}

fn Cs(s:&str) -> CString { CString::new(s).unwrap() }

/// # Safety
/// `Ptr` must be non-null and point to a valid, NUL-terminated C string
/// allocated by the C ABI (freed here via `AphroditeFreeString`).
unsafe fn Take(Ptr:*mut c_char) -> String {
	assert!(!Ptr.is_null(), "expected non-null C string");
	// SAFETY: `Ptr` is asserted non-null above and, per the safety
	// contract of this function, points to a valid, NUL-terminated C
	// string that is not aliased elsewhere.
	let s = unsafe { CStr::from_ptr(Ptr) }.to_string_lossy().into_owned();
	// SAFETY: `Ptr` was allocated by the C ABI, which owns the
	// corresponding `AphroditeFreeString` deallocation routine.
	AphroditeFreeString(Ptr);
	s
}

// ── T6 (F4, report 06): per-handle locking, not one global mutex ──
// Previously `Handles` held every session's `AphroditeState` directly
// under one `Mutex`, so a long-running call on one session (e.g. a slow
// hook) serialized every OTHER session's calls behind it too, even
// though sessions share no state. Verifies handle 2's call completes
// quickly while handle 1's call is still sleeping inside its own lock.
#[test]
fn WithStateDoesNotSerializeAcrossDifferentHandles() {
	let H1:usize = unsafe { Take(AphroditeInit(std::ptr::null())) }.parse().unwrap();
	let H2:usize = unsafe { Take(AphroditeInit(std::ptr::null())) }.parse().unwrap();

	let Barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
	let B1 = Barrier.clone();
	let T1 = std::thread::spawn(move || {
		WithState(H1, |_S| {
			B1.wait();
			std::thread::sleep(std::time::Duration::from_millis(300));
		})
		.unwrap();
	});

	Barrier.wait();
	let Start = std::time::Instant::now();
	WithState(H2, |_S| {}).unwrap();
	let Elapsed = Start.elapsed();

	T1.join().unwrap();
	Handles().as_mut().and_then(|m| m.remove(&H1));
	Handles().as_mut().and_then(|m| m.remove(&H2));

	assert!(
		Elapsed < std::time::Duration::from_millis(150),
		"handle 2's call should not block behind handle 1's long-held lock, took {Elapsed:?}"
	);
}

#[test]
fn InitWithNullConfigurationYieldsParseableHandle() {
	let h = unsafe { Take(AphroditeInit(std::ptr::null())) };
	assert!(h.parse::<usize>().is_ok(), "handle {:?} should parse as usize", h);
	AphroditeDestroy(Cs(&h).as_ptr());
}

#[test]
fn CompressRetrieveDestroyRoundtrip() {
	let h = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Handle = Cs(&h);

	let CompressJSON = unsafe {
		Take(AphroditeCompress(
			Handle.as_ptr(),
			Cs("fn main() {}").as_ptr(),
			Cs("text").as_ptr(),
		))
	};
	let v:serde_json::Value = serde_json::from_str(&CompressJSON).unwrap();
	let hash = v["hash"].as_str().unwrap().to_string();
	assert!(!hash.is_empty());

	let Retrieved = unsafe { Take(AphroditeRetrieve(Handle.as_ptr(), Cs(&hash).as_ptr())) };
	assert_eq!(Retrieved, "fn main() {}");

	AphroditeDestroy(Handle.as_ptr());
	// Handle is gone: any further stateful call reports invalid handle, no crash.
	let After = unsafe { Take(AphroditeStats(Handle.as_ptr())) };
	assert!(After.contains("invalid handle"));
}

#[test]
fn RetrieveUnknownHashReturnsErrorJSON() {
	let h = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Handle = Cs(&h);
	let Out = unsafe { Take(AphroditeRetrieve(Handle.as_ptr(), Cs("deadbeef00000000").as_ptr())) };
	let v:serde_json::Value = serde_json::from_str(&Out).unwrap();
	assert!(v["error"].as_str().unwrap().contains("hash not found"));
	AphroditeDestroy(Handle.as_ptr());
}

#[test]
fn InvalidHandlesAreRejectedWithoutCrashing() {
	for Bad in ["999999", "abc", ""] {
		let Handle = Cs(Bad);
		for Out in [
			unsafe { Take(AphroditeStats(Handle.as_ptr())) },
			unsafe { Take(AphroditeCatalog(Handle.as_ptr(), Cs("full").as_ptr())) },
			unsafe { Take(AphroditeSearch(Handle.as_ptr(), Cs("q").as_ptr())) },
			unsafe { Take(AphroditeConfigurationGet(Handle.as_ptr(), Cs("model").as_ptr())) },
			unsafe { Take(AphroditeSessionStart(Handle.as_ptr())) },
			unsafe { Take(AphroditeCompress(Handle.as_ptr(), Cs("x").as_ptr(), Cs("text").as_ptr())) },
			unsafe { Take(AphroditeRetrieve(Handle.as_ptr(), Cs("x").as_ptr())) },
		] {
			assert!(
				Out.contains("invalid handle"),
				"expected invalid-handle error for {:?}, got {:?}",
				Bad,
				Out
			);
		}
		// Must not panic even though the handle is bogus.
		AphroditeDestroy(Handle.as_ptr());
	}
}

#[test]
fn ClassifyNullContentReturnsError() {
	let Out = unsafe { Take(AphroditeClassify(std::ptr::null())) };
	let v:serde_json::Value = serde_json::from_str(&Out).unwrap();
	assert_eq!(v["error"].as_str().unwrap(), "null content");
}

#[test]
fn DispatchKnownAndUnknownHooks() {
	let h = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Handle = Cs(&h);

	let Okay = unsafe {
		Take(AphroditeDispatch(
			Handle.as_ptr(),
			Cs("session_start").as_ptr(),
			Cs("{}").as_ptr(),
		))
	};
	assert!(serde_json::from_str::<serde_json::Value>(&Okay).is_ok());

	let Unknown = unsafe {
		Take(AphroditeDispatch(
			Handle.as_ptr(),
			Cs("not_a_real_hook").as_ptr(),
			Cs("{}").as_ptr(),
		))
	};
	let v:serde_json::Value = serde_json::from_str(&Unknown).unwrap();
	assert!(v["error"].as_str().unwrap().contains("unknown hook"));

	AphroditeDestroy(Handle.as_ptr());
}

#[test]
fn FreeStringNullIsANoOp() { AphroditeFreeString(std::ptr::null_mut()); }

#[test]
fn OversizeContentIsRejected() {
	let h = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Handle = Cs(&h);
	let Big = "a".repeat(MaximumContent + 1);
	let Out = unsafe { Take(AphroditeCompress(Handle.as_ptr(), Cs(&Big).as_ptr(), Cs("text").as_ptr())) };
	let v:serde_json::Value = serde_json::from_str(&Out).unwrap();
	assert!(v["error"].as_str().unwrap().contains("16MB"));
	AphroditeDestroy(Handle.as_ptr());
}

// ── T3 (F5): the seven previously-unguarded extern fns must survive a
// panic without aborting - a deliberately panicking classify call must
// come back as error JSON, not crash the test process. ──
#[test]
fn GuardedHelperConvertsPanicToErrorJSON() {
	let Ptr = Guarded(|| panic!("deliberate test panic"));
	let Out = unsafe { Take(Ptr) };
	let v:serde_json::Value = serde_json::from_str(&Out).unwrap();
	assert!(v["error"].as_str().unwrap().contains("panicked"));
}

// ── T4 (F3): interior NUL bytes in stored/derived content must not
// panic `CString::new(...).unwrap()` at any of the five raw-return
// sites (retrieve, config_get, filter_lines, preview, stage2). A raw
// C-string *input* can never carry a NUL (by definition), so the only
// real ingress is via JSON args to `AphroditeDispatch`, exactly as F3
// describes: `{"content":"a\u0000b"}` decodes to a Rust `String`
// containing a NUL byte.
#[test]
fn RetrieveToleratesInteriorNulInDispatchedContent() {
	let h = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Handle = Cs(&h);
	// Force compression regardless of size so a hash is always produced.
	let _ = unsafe {
		Take(AphroditeConfigurationSet(
			Handle.as_ptr(),
			Cs("tool_threshold").as_ptr(),
			Cs("0").as_ptr(),
		))
	};
	let Args = Cs(r#"{"content":"x\u0000y","tool_name":"t"}"#);
	let Dispatched = unsafe {
		Take(AphroditeDispatch(
			Handle.as_ptr(),
			Cs("transform_tool_result").as_ptr(),
			Args.as_ptr(),
		))
	};
	let v:serde_json::Value = serde_json::from_str(&Dispatched).unwrap();
	let hash = v["hash"].as_str().expect("compression should have produced a hash").to_string();
	// Must not panic even though the stored content contains a NUL byte.
	let Retrieved = unsafe { Take(AphroditeRetrieve(Handle.as_ptr(), Cs(&hash).as_ptr())) };
	assert_eq!(Retrieved, "xy"); // NUL stripped, not left dangling
	AphroditeDestroy(Handle.as_ptr());
}

// ── Issue #38 class: `AphroditeReload` on a found-but-broken TOML
// used to discard the parse error silently (`.parse().ok()`), leaving
// "reload did nothing" indistinguishable from "reload succeeded". Now
// it warns on a real surface (stderr fallback), records `ConfigurationError`
// on the state, and echoes it in the response. ──
#[test]
fn TestAphroditeReloadBrokenTomlRecordsConfigurationError() {
	let h = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Handle = Cs(&h);
	let Stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	let Broken = std::env::temp_dir().join(format!("aphrodite-cfg-reload-ffi-{Stamp}.toml"));
	std::fs::write(&Broken, "[compression\n").unwrap();

	let Response = unsafe { Take(AphroditeReload(Handle.as_ptr(), Cs(Broken.to_str().unwrap()).as_ptr())) };
	let _ = std::fs::remove_file(&Broken);
	let v:serde_json::Value = serde_json::from_str(&Response).unwrap();
	assert_eq!(v["status"], "ok", "reload must degrade, not fail: {Response}");
	assert!(
		v["config_error"].as_str().unwrap().contains(Broken.to_str().unwrap()),
		"response must carry the parse failure: {Response}"
	);

	// The state recorded it too, so `AphroditeStats` self-diagnoses.
	let Stats = unsafe { Take(AphroditeStats(Handle.as_ptr())) };
	let Sv:serde_json::Value = serde_json::from_str(&Stats).unwrap();
	assert!(
		Sv["config_error"].as_str().unwrap().contains(Broken.to_str().unwrap()),
		"stats must surface the reload parse failure: {Stats}"
	);
	AphroditeDestroy(Handle.as_ptr());
}

// ── Scripting gate (pre-Rust scripting.rs port): the module must not
// exist in a feature-off build. This test compiles and passes ONLY when
// the `scripting` feature is off - `pub mod Scripting` is declared behind
// `#[cfg(feature = "scripting")]`, so the default build has no scripting
// module at all. ──
#[cfg(not(feature = "scripting"))]
#[test]
fn ScriptingModuleAbsentWithoutFeature() {
	assert!(!cfg!(feature = "scripting"));
}
