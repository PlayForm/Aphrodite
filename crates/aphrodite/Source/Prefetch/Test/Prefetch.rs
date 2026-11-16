//! The prefetch call surface: missing files, real files, the size cap and the
//! inline-store budget the response reports.

use crate::{
	Prefetch::{Files::Fn::Fn as Files, Read::Const::Maximum},
	State::Struct::AphroditeState,
};

#[test]
fn Missing() {
	let mut s = AphroditeState::default();
	let r = Files(&mut s, &["/nonexistent/file/xyzzy.txt".to_string()]);
	assert_eq!(r["missing"], 1);
	assert_eq!(r["loaded"], 0);
}

#[test]
fn Real() {
	let mut s = AphroditeState::default();
	let Src = env!("CARGO_MANIFEST_DIR").to_string() + "/Source/Prefetch/mod.rs";
	let r = Files(&mut s, std::slice::from_ref(&Src));
	assert_eq!(r["loaded"], 1, "prefetch failed: {:?}", r);
	assert_eq!(s.Marker.Recent.len(), 1);
}

// ── T11 (F11): prefetch surfaces the inline store's byte budget ──
#[test]
fn Budget() {
	let mut s = AphroditeState::default();
	let Src = env!("CARGO_MANIFEST_DIR").to_string() + "/Source/Prefetch/mod.rs";
	let r = Files(&mut s, &[Src]);
	assert_eq!(r["inline_store_byte_budget"], 256 * 1024 * 1024);
	assert!(r["inline_store_bytes"].as_u64().unwrap() > 0);
}

#[test]
fn Oversize() {
	// Use a real temp file over MAX_PREFETCH_SIZE rather than mocking
	// metadata, to exercise the actual `Paths` size check.
	let Directory = std::env::temp_dir();
	let path = Directory.join(format!("aphrodite_prefetch_oversize_test_{}.txt", std::process::id()));
	std::fs::write(&path, vec![b'x'; (Maximum + 1) as usize]).unwrap();

	let mut s = AphroditeState::default();
	let r = Files(&mut s, &[path.to_string_lossy().to_string()]);
	assert_eq!(r["skipped_size"], 1);
	let Reason = r["results"][0]["reason"].as_str().unwrap();
	assert!(
		Reason.contains(&Maximum.to_string()),
		"reason should surface the byte limit: {Reason}"
	);

	let _ = std::fs::remove_file(&path);
}

#[test]
fn Multiple() {
	let mut s = AphroditeState::default();
	let Src = env!("CARGO_MANIFEST_DIR").to_string() + "/Source/Prefetch/mod.rs";
	let r = Files(&mut s, &[Src, "/nonexistent/abc".to_string()]);
	assert_eq!(r["loaded"], 1);
	assert_eq!(r["missing"], 1);
}
