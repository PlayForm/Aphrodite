//! Crash-regression tests: every pipeline stage must survive every corpus
//! fixture - especially the pathological ones (wave-1 UTF-8 slice-panic
//! class, 05-F1 literal-marker corruption class, 03-F3 interior-NUL class).

use aphrodite_bench_compression::{load_corpus, pathological_corpus, run_all_stages};

#[test]
fn no_stage_panics_on_any_corpus_fixture() {
	for f in load_corpus() {
		let content = f.content.clone();
		let result = std::panic::catch_unwind(move || run_all_stages(&content));
		assert!(result.is_ok(), "pipeline stage panicked on fixture {}", f.name);
	}
}

#[test]
fn pathological_fixtures_are_present_and_flagged() {
	let p = pathological_corpus();
	let names: Vec<&str> = p.iter().map(|f| f.name.as_str()).collect();
	assert!(names.contains(&"utf8_boundary.txt"));
	assert!(names.contains(&"marker_literal.txt"));
	assert!(names.contains(&"interior_nul.json"));
}

#[test]
fn utf8_boundary_fixture_straddles_500_byte_marks() {
	// The fixture is only a regression corpus if 500-byte offsets really do
	// land mid-codepoint (the wave-1 slice-panic precondition).
	let f = pathological_corpus().into_iter().find(|f| f.name == "utf8_boundary.txt").unwrap();
	let bytes = f.content.as_bytes();
	let straddles = (500..bytes.len()).step_by(500).filter(|&i| !f.content.is_char_boundary(i)).count();
	assert!(straddles > 0, "no 500-byte offset lands inside a codepoint - fixture lost its teeth");
}

#[test]
fn expand_preserves_literal_markers_byte_identical() {
	// 05-F1 corruption class: stored content that merely CONTAINS
	// marker-shaped text must round-trip byte-identical through recursive
	// expansion (none of the embedded hashes exist in the store).
	let f = pathological_corpus().into_iter().find(|f| f.name == "marker_literal.txt").unwrap();
	let mut state = aphrodite::state::AphroditeState::default();
	state.inline_store_put("aaaabbbbccccddddeeeeffff".into(), f.content.clone());
	let expanded = aphrodite::resolve::expand(&mut state, "aaaabbbbccccddddeeeeffff").expect("found");
	assert_eq!(expanded, f.content, "literal <<<CCR:...>>> text was corrupted by expansion");
	// And the pristine copy must still be stored (no write-back).
	assert_eq!(aphrodite::resolve::resolve_one(&mut state, "aaaabbbbccccddddeeeeffff"), Some(f.content));
}

#[test]
fn interior_nul_content_round_trips_through_inline_store() {
	// 03-F3 class: the raw store path (unlike the C-ABI ingress, which
	// rejects NULs) must hold and return NUL-bearing content unmodified.
	let f = pathological_corpus().into_iter().find(|f| f.name == "interior_nul.json").unwrap();
	assert!(f.content.contains('\0'), "fixture must actually contain NUL bytes");
	let key = headroom_core::ccr::compute_key(f.content.as_bytes());
	let mut state = aphrodite::state::AphroditeState::default();
	state.inline_store_put(key.clone(), f.content.clone());
	assert_eq!(state.inline_store_get(&key), Some(f.content));
}

#[test]
fn filter_lines_survives_pathological_corpus() {
	for f in pathological_corpus() {
		let content = f.content.clone();
		let r = std::panic::catch_unwind(move || {
			let hit = aphrodite::resolve::filter_lines(&content, "CCR");
			let miss = aphrodite::resolve::filter_lines(&content, "zz-no-such-line-zz");
			hit.len() + miss.len()
		});
		assert!(r.is_ok(), "filter_lines panicked on {}", f.name);
	}
}