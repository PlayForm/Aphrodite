//! The ABI battery: every guarantee the C surface makes, asserted through the
//! exported symbols themselves rather than through the internals behind them.
//!
//! These tests call the `extern "C"` functions exactly as a host would - raw
//! pointers in, owned C strings out, freed with `AphroditeFreeString` - which
//! is the only way to catch a contract break that the Rust-typed call path would
//! paper over.
//!
//! Each test is named for the one guarantee it covers; the module path names the
//! surface (`ABI::Test`).

use std::{
	ffi::{CStr, CString},
	os::raw::c_char,
};

use crate::{
	ABI::{
		Boundary::Run::Fn::Fn as Run,
		Catalog::AphroditeCatalog,
		Classify::AphroditeClassify,
		Compress::AphroditeCompress,
		ConfigurationGet::AphroditeConfigurationGet,
		ConfigurationSet::AphroditeConfigurationSet,
		Destroy::AphroditeDestroy,
		Dispatch::AphroditeDispatch,
		FreeString::AphroditeFreeString,
		Guard::Content::Const::Maximum,
		Handle::{Lookup::Guarded, Registry::Const::Handles},
		Initialise::AphroditeInit,
		Reload::AphroditeReload,
		Retrieve::AphroditeRetrieve,
		Search::AphroditeSearch,
		SessionStart::AphroditeSessionStart,
		Stats::AphroditeStats,
	},
	Preview::Builder::Entry::Fn as Build,
};

// ── T9 (F10): the preview builder's "terminal" arm ────────────
// `Hook::Terminal` overrides the classified type to "terminal" for
// exit-code/error-shaped output, but `Build` had no matching arm, so the
// preview silently fell through to the generic `_` branch (a bare line/byte
// count with no exit-code context).

/// A terminal preview surfaces the exit-code line.
#[test]
fn Exit() {
	let _Guard = crate::Preview::State::Guard::Fn();
	let Preview = Build("terminal", "running tests\nall good\nexit code: 1\n");
	assert!(Preview.starts_with("[terminal:"));
	assert!(
		Preview.contains("exit code: 1"),
		"preview should surface the exit code line: {Preview}"
	);
}

/// ISSUE-11 residual #2: the terminal-arm fallback used to be the LAST non-empty
/// line - a multi-line block previewed as its closing brace (`[terminal:7L }]`).
/// The FIRST meaningful line is the honest default (skipping lone braces).
#[test]
fn Fallback() {
	let _Guard = crate::Preview::State::Guard::Fn();
	let Preview = Build("terminal", "line one\nline two\nlast line here\n");
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

/// A NUL-terminated C string built from `Text`.
fn Literal(Text:&str) -> CString { CString::new(Text).unwrap() }

/// # Safety
/// `Ptr` must be non-null and point to a valid, NUL-terminated C string
/// allocated by the C ABI (freed here via `AphroditeFreeString`).
unsafe fn Take(Ptr:*mut c_char) -> String {
	assert!(!Ptr.is_null(), "expected non-null C string");
	// SAFETY: `Ptr` is asserted non-null above and, per the safety
	// contract of this function, points to a valid, NUL-terminated C
	// string that is not aliased elsewhere.
	let Text = unsafe { CStr::from_ptr(Ptr) }.to_string_lossy().into_owned();
	// SAFETY: `Ptr` was allocated by the C ABI, which owns the
	// corresponding `AphroditeFreeString` deallocation routine.
	unsafe { AphroditeFreeString(Ptr) };
	Text
}

// ── T6 (F4, report 06): per-handle locking, not one global mutex ──
// Previously `Handles` held every session's state directly under one `Mutex`, so
// a long-running call on one session (e.g. a slow hook) serialized every OTHER
// session's calls behind it too, even though sessions share no state.

/// Handle two's call completes quickly while handle one's call still sleeps
/// inside its own lock.
#[test]
fn Concurrency() {
	let First:usize = unsafe { Take(AphroditeInit(std::ptr::null())) }.parse().unwrap();
	let Second:usize = unsafe { Take(AphroditeInit(std::ptr::null())) }.parse().unwrap();

	let Barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
	let Arrived = Barrier.clone();
	let Worker = std::thread::spawn(move || {
		Guarded(First, |_State| {
			Arrived.wait();
			std::thread::sleep(std::time::Duration::from_millis(300));

			Ok(())
		})
		.unwrap();
	});

	Barrier.wait();
	let Start = std::time::Instant::now();
	Guarded(Second, |_State| Ok(())).unwrap();
	let Elapsed = Start.elapsed();

	Worker.join().unwrap();
	Handles.Remove(First);
	Handles.Remove(Second);

	assert!(
		Elapsed < std::time::Duration::from_millis(150),
		"handle 2's call should not block behind handle 1's long-held lock, took {Elapsed:?}"
	);
}

/// A null configuration path still yields a handle that parses as a decimal.
#[test]
fn Initialise() {
	let Decimal = unsafe { Take(AphroditeInit(std::ptr::null())) };
	assert!(Decimal.parse::<usize>().is_ok(), "handle {Decimal:?} should parse as usize",);
	unsafe { AphroditeDestroy(Literal(&Decimal).as_ptr()) };
}

/// Compress, retrieve, destroy: the round trip a host performs, and the
/// invalid-handle answer once the session is gone.
#[test]
fn Roundtrip() {
	let Decimal = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Pointer = Literal(&Decimal);

	let Response = unsafe {
		Take(AphroditeCompress(
			Pointer.as_ptr(),
			Literal("fn main() {}").as_ptr(),
			Literal("text").as_ptr(),
		))
	};
	let Body:serde_json::Value = serde_json::from_str(&Response).unwrap();
	let Hash = Body["hash"].as_str().unwrap().to_string();
	assert!(!Hash.is_empty());

	let Content = unsafe { Take(AphroditeRetrieve(Pointer.as_ptr(), Literal(&Hash).as_ptr())) };
	assert_eq!(Content, "fn main() {}");

	unsafe { AphroditeDestroy(Pointer.as_ptr()) };
	// The handle is gone: any further stateful call reports invalid handle, no crash.
	let Failure = unsafe { Take(AphroditeStats(Pointer.as_ptr())) };
	assert!(Failure.contains("invalid handle"));
}

/// An unknown hash answers error JSON rather than a dangling pointer.
#[test]
fn Unknown() {
	let Decimal = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Pointer = Literal(&Decimal);
	let Answer = unsafe { Take(AphroditeRetrieve(Pointer.as_ptr(), Literal("deadbeef00000000").as_ptr())) };
	let Body:serde_json::Value = serde_json::from_str(&Answer).unwrap();
	assert!(Body["error"].as_str().unwrap().contains("hash not found"));
	unsafe { AphroditeDestroy(Pointer.as_ptr()) };
}

/// Every stateful export answers "invalid handle" for a bogus handle and none
/// of them crashes.
#[test]
fn Invalid() {
	["999999", "abc", ""].into_iter().for_each(|Bad| {
		let Pointer = Literal(Bad);
		[
			unsafe { Take(AphroditeStats(Pointer.as_ptr())) },
			unsafe { Take(AphroditeCatalog(Pointer.as_ptr(), Literal("full").as_ptr())) },
			unsafe { Take(AphroditeSearch(Pointer.as_ptr(), Literal("q").as_ptr())) },
			unsafe { Take(AphroditeConfigurationGet(Pointer.as_ptr(), Literal("model").as_ptr())) },
			unsafe { Take(AphroditeSessionStart(Pointer.as_ptr())) },
			unsafe {
				Take(AphroditeCompress(
					Pointer.as_ptr(),
					Literal("x").as_ptr(),
					Literal("text").as_ptr(),
				))
			},
			unsafe { Take(AphroditeRetrieve(Pointer.as_ptr(), Literal("x").as_ptr())) },
		]
		.into_iter()
		.for_each(|Answer| {
			assert!(
				Answer.contains("invalid handle"),
				"expected invalid-handle error for {:?}, got {:?}",
				Bad,
				Answer
			);
		});
		// Must not panic even though the handle is bogus.
		unsafe { AphroditeDestroy(Pointer.as_ptr()) };
	});
}

/// Null content is refused by name.
#[test]
fn Null() {
	let Answer = unsafe { Take(AphroditeClassify(std::ptr::null())) };
	let Body:serde_json::Value = serde_json::from_str(&Answer).unwrap();
	assert_eq!(Body["error"].as_str().unwrap(), "null content");
}

/// A served hook answers JSON; an unserved one answers an unknown-hook error.
#[test]
fn Dispatch() {
	let Decimal = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Pointer = Literal(&Decimal);

	let Known = unsafe {
		Take(AphroditeDispatch(
			Pointer.as_ptr(),
			Literal("session_start").as_ptr(),
			Literal("{}").as_ptr(),
		))
	};
	assert!(serde_json::from_str::<serde_json::Value>(&Known).is_ok());

	let Body = unsafe {
		Take(AphroditeDispatch(
			Pointer.as_ptr(),
			Literal("not_a_real_hook").as_ptr(),
			Literal("{}").as_ptr(),
		))
	};
	let Parsed:serde_json::Value = serde_json::from_str(&Body).unwrap();
	assert!(Parsed["error"].as_str().unwrap().contains("unknown hook"));

	unsafe { AphroditeDestroy(Pointer.as_ptr()) };
}

/// Freeing a null pointer is the documented no-op.
#[test]
fn Free() {
	// SAFETY: null is the documented no-op case for the deallocator.
	unsafe { AphroditeFreeString(std::ptr::null_mut()) };
}

/// Content above the 16 MB cap is refused by name.
#[test]
fn Oversize() {
	let Decimal = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Pointer = Literal(&Decimal);
	let Payload = "a".repeat(Maximum + 1);
	let Answer = unsafe {
		Take(AphroditeCompress(
			Pointer.as_ptr(),
			Literal(&Payload).as_ptr(),
			Literal("text").as_ptr(),
		))
	};
	let Body:serde_json::Value = serde_json::from_str(&Answer).unwrap();
	assert!(Body["error"].as_str().unwrap().contains("16MB"));
	unsafe { AphroditeDestroy(Pointer.as_ptr()) };
}

// ── T3 (F5): the seven previously-unguarded extern fns must survive a
// panic without aborting - a deliberately panicking call must come back
// as error JSON, not crash the test process. ──

/// A panic inside the boundary comes back as error JSON.
#[test]
fn Panic() {
	let Ptr = Run(|| panic!("deliberate test panic"));
	let Answer = unsafe { Take(Ptr) };
	let Body:serde_json::Value = serde_json::from_str(&Answer).unwrap();
	assert!(Body["error"].as_str().unwrap().contains("panicked"));
}

// ── T4 (F3): interior NUL bytes in stored/derived content must not
// panic `CString::new(...).unwrap()` at any of the five raw-return
// sites (retrieve, config_get, filter_lines, preview, stage2). A raw
// C-string *input* can never carry a NUL (by definition), so the only
// real ingress is via JSON args to `AphroditeDispatch`, exactly as F3
// describes: `{"content":"a\u0000b"}` decodes to a Rust `String`
// containing a NUL byte.

/// A stored payload carrying an interior NUL is answered with the NUL stripped,
/// not with a dangling pointer or a truncated string.
#[test]
fn Interior() {
	let Decimal = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Pointer = Literal(&Decimal);
	// Force compression regardless of size so a hash is always produced.
	let _ = unsafe {
		Take(AphroditeConfigurationSet(
			Pointer.as_ptr(),
			Literal("tool_threshold").as_ptr(),
			Literal("0").as_ptr(),
		))
	};
	let Args = Literal(r#"{"content":"x\u0000y","tool_name":"t"}"#);
	let Response = unsafe {
		Take(AphroditeDispatch(
			Pointer.as_ptr(),
			Literal("transform_tool_result").as_ptr(),
			Args.as_ptr(),
		))
	};
	let Body:serde_json::Value = serde_json::from_str(&Response).unwrap();
	let Hash = Body["hash"]
		.as_str()
		.expect("compression should have produced a hash")
		.to_string();
	// Must not panic even though the stored content contains a NUL byte.
	let Content = unsafe { Take(AphroditeRetrieve(Pointer.as_ptr(), Literal(&Hash).as_ptr())) };
	assert_eq!(Content, "xy"); // NUL stripped, not left dangling
	unsafe { AphroditeDestroy(Pointer.as_ptr()) };
}

// ── Issue #38 class: `AphroditeReload` on a found-but-broken TOML
// used to discard the parse error silently (`.parse().ok()`), leaving
// "reload did nothing" indistinguishable from "reload succeeded". Now
// it warns on a real surface (stderr fallback), records the configuration
// error on the state, and echoes it in the response. ──

/// A broken TOML file degrades the reload to `{"status":"ok"}` plus the parse
/// failure, and the failure is also visible through `AphroditeStats`.
#[test]
fn Reload() {
	let Decimal = unsafe { Take(AphroditeInit(std::ptr::null())) };
	let Pointer = Literal(&Decimal);
	let Stamp = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_nanos();
	let Broken = std::env::temp_dir().join(format!("aphrodite-cfg-reload-ffi-{Stamp}.toml"));
	std::fs::write(&Broken, "[compression\n").unwrap();

	let Response = unsafe { Take(AphroditeReload(Pointer.as_ptr(), Literal(Broken.to_str().unwrap()).as_ptr())) };
	let _ = std::fs::remove_file(&Broken);
	let Body:serde_json::Value = serde_json::from_str(&Response).unwrap();
	assert_eq!(Body["status"], "ok", "reload must degrade, not fail: {Response}");
	assert!(
		Body["config_error"].as_str().unwrap().contains(Broken.to_str().unwrap()),
		"response must carry the parse failure: {Response}"
	);

	// The state recorded it too, so `AphroditeStats` self-diagnoses.
	let Stats = unsafe { Take(AphroditeStats(Pointer.as_ptr())) };
	let Value:serde_json::Value = serde_json::from_str(&Stats).unwrap();
	assert!(
		Value["config_error"].as_str().unwrap().contains(Broken.to_str().unwrap()),
		"stats must surface the reload parse failure: {Stats}"
	);
	unsafe { AphroditeDestroy(Pointer.as_ptr()) };
}
