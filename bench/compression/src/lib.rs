//! Shared corpus-loading helpers for the standalone compression bench suite.
//!
//! The corpus lives in `bench/corpus/` (sibling of this crate). Fixtures
//! marked pathological double as crash-regression inputs: every pipeline
//! stage must run over them without panicking.

use std::path::PathBuf;

/// Absolute path to `bench/corpus/`.
pub fn corpus_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus").canonicalize().expect("corpus dir exists")
}

/// One corpus fixture: file name + UTF-8 content.
#[derive(Debug, Clone)]
pub struct Fixture {
	pub name: String,
	pub content: String,
	/// Pathological fixtures are crash-regression inputs (wave-1 classes:
	/// UTF-8 slice panics, literal-marker corruption, interior NULs).
	pub pathological: bool,
}

const PATHOLOGICAL: &[&str] = &["utf8_boundary.txt", "marker_literal.txt", "interior_nul.json"];

/// Load every corpus fixture (excluding the generator script), sorted by name.
pub fn load_corpus() -> Vec<Fixture> {
	let mut out = Vec::new();
	for entry in std::fs::read_dir(corpus_dir()).expect("read corpus dir") {
		let entry = entry.expect("dir entry");
		let name = entry.file_name().to_string_lossy().into_owned();
		// Skip the generator/verifier scripts, the corpus README, dotfiles,
		// and subdirectories (e.g. __pycache__ left by gen_corpus.py).
		if name == "gen_corpus.py" || name == "verify_labels.py" || name == "README.md" || name.starts_with('.') {
			continue;
		}
		if entry.file_type().expect("file type").is_dir() {
			continue;
		}
		let bytes = std::fs::read(entry.path()).expect("read fixture");
		// interior_nul.json carries raw 0x00 bytes; all fixtures are valid UTF-8.
		let content = String::from_utf8(bytes).expect("fixtures are valid UTF-8");
		out.push(Fixture { pathological: PATHOLOGICAL.contains(&name.as_str()), name, content });
	}
	out.sort_by(|a, b| a.name.cmp(&b.name));
	assert!(out.len() >= 10, "expected at least 10 corpus fixtures, got {}", out.len());
	out
}

/// The pathological subset.
pub fn pathological_corpus() -> Vec<Fixture> {
	load_corpus().into_iter().filter(|f| f.pathological).collect()
}

/// Run the full non-proxy pipeline stage set over one content blob,
/// returning a fingerprint (so callers can black_box it). Panics propagate -
/// callers wrap in catch_unwind where a panic is the failure being tested.
pub fn run_all_stages(content: &str) -> usize {
	let ty = aphrodite::detect_type(content);
	let preview = aphrodite::build_preview(&ty, content);
	let stage2 = aphrodite::stage2::compress_stage2(content, &ty);
	let structure = aphrodite::struct_extract::extract_code_structure(content, "");
	let hashes = aphrodite::marker::extract_hashes(content);
	let key = headroom_core::ccr::compute_key(content.as_bytes());
	let marker = aphrodite::marker::ccr_marker(&key, &ty, content.len(), &preview, Some(80), None, None);
	let parsed = aphrodite::marker::parse_preview(marker.lines().nth(1).unwrap_or(""));
	ty.len()
		+ preview.len()
		+ stage2.map(|s| s.len()).unwrap_or(0)
		+ structure.len()
		+ hashes.len()
		+ marker.len()
		+ parsed.map(|p| p.len()).unwrap_or(0)
}