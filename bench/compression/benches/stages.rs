//! Stage-level micro-benchmarks for the aphrodite compression pipeline.
//!
//! Run with `cargo bench` (full) or `cargo bench -- --quick` (reduced
//! sampling, used for the recorded baseline). Groups:
//!   classify        - content-type detection over the whole corpus
//!   stage2          - semantic reduction per reducible content type
//!   struct_extract  - code structure extraction per language
//!   marker          - CCR marker generate + preview parse + hash extract
//!   resolve         - nested marker expansion, 1-5 levels deep
//!   store           - inline store put/get + content-address hashing
//!   pathological    - full stage set over crash-regression fixtures
//!                     (a panic anywhere fails the bench run)

use std::collections::HashMap;

use aphrodite_bench_compression::{load_corpus, pathological_corpus, run_all_stages};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

fn bench_classify(c:&mut Criterion) {
	let corpus = load_corpus();
	let mut g = c.benchmark_group("classify");
	for f in &corpus {
		g.throughput(Throughput::Bytes(f.content.len() as u64));
		g.bench_with_input(BenchmarkId::from_parameter(&f.name), &f.content, |b, content| {
			b.iter(|| aphrodite::detect_type(black_box(content)));
		});
	}
	g.finish();
}

fn bench_stage2(c:&mut Criterion) {
	let corpus = load_corpus();
	// Files whose detected type has a stage-2 reducer, plus text as control.
	let picks = [
		"build_log.txt",
		"git_diff.patch",
		"wide_5k_keys.json",
		"code_rust.rs",
		"text_100kb.txt",
	];
	let mut g = c.benchmark_group("stage2");
	for f in corpus.iter().filter(|f| picks.contains(&f.name.as_str())) {
		let ty = aphrodite::detect_type(&f.content);
		g.throughput(Throughput::Bytes(f.content.len() as u64));
		g.bench_with_input(
			BenchmarkId::from_parameter(format!("{}[{}]", f.name, ty)),
			&f.content,
			|b, content| {
				b.iter(|| aphrodite::stage2::compress_stage2(black_box(content), black_box(&ty)));
			},
		);
	}
	g.finish();
}

fn bench_struct_extract(c:&mut Criterion) {
	let corpus = load_corpus();
	let langs = [
		("code_rust.rs", "rust"),
		("code_python.py", "python"),
		("code_go.go", "go"),
		("code_ts.ts", "ts"),
	];
	let mut g = c.benchmark_group("struct_extract");
	for (file, lang) in langs {
		let f = corpus.iter().find(|f| f.name == file).expect("language fixture present");
		g.throughput(Throughput::Bytes(f.content.len() as u64));
		g.bench_with_input(BenchmarkId::from_parameter(lang), &f.content, |b, content| {
			b.iter(|| aphrodite::struct_extract::extract_code_structure(black_box(content), black_box(lang)));
		});
	}
	g.finish();
}

fn bench_marker(c:&mut Criterion) {
	let corpus = load_corpus();
	let rust = &corpus.iter().find(|f| f.name == "code_rust.rs").unwrap().content;
	let ty = aphrodite::detect_type(rust);
	let preview = aphrodite::build_preview(&ty, rust);
	let key = headroom_core::ccr::compute_key(rust.as_bytes());
	let mut meta = HashMap::new();
	meta.insert("tool".to_string(), "read_file".to_string());
	meta.insert("path".to_string(), "src/lib.rs".to_string());

	let mut g = c.benchmark_group("marker");
	g.bench_function("generate", |b| {
		b.iter(|| {
			aphrodite::marker::ccr_marker(
				black_box(&key),
				black_box(&ty),
				rust.len(),
				black_box(&preview),
				Some(80),
				Some(&meta),
				None,
			)
		});
	});
	let marker = aphrodite::marker::ccr_marker(&key, &ty, rust.len(), &preview, Some(80), Some(&meta), None);
	let preview_line = marker.lines().nth(1).unwrap().to_string();
	g.bench_function("parse_preview", |b| {
		b.iter(|| aphrodite::marker::parse_preview(black_box(&preview_line)));
	});
	let marker_doc = &corpus.iter().find(|f| f.name == "marker_literal.txt").unwrap().content;
	g.throughput(Throughput::Bytes(marker_doc.len() as u64));
	g.bench_function("extract_hashes_10kb_doc", |b| {
		b.iter(|| aphrodite::marker::extract_hashes(black_box(marker_doc)));
	});
	g.finish();
}

fn bench_resolve(c:&mut Criterion) {
	// Chain of nested markers: lvl0 -> lvl1 -> ... -> lvlN (leaf).
	// Depth d means d marker hops before hitting plain content.
	let mut g = c.benchmark_group("resolve");
	for depth in 1..=5usize {
		let mut state = aphrodite::state::AphroditeState::default();
		let padding = "context line before\n".repeat(20);
		for lvl in 0..depth {
			let child = format!("<<<CCR:lvl{}{}|text|100>>>", lvl + 1, "0".repeat(30));
			state.inline_store_put(
				format!("lvl{}{}", lvl, "0".repeat(30)),
				format!("{padding}embedded {child} tail\n"),
			);
		}
		state.inline_store_put(format!("lvl{}{}", depth, "0".repeat(30)), "LEAF CONTENT\n".repeat(10));
		let root = format!("lvl0{}", "0".repeat(30));
		g.bench_with_input(BenchmarkId::from_parameter(format!("nested_{depth}")), &root, |b, root| {
			b.iter(|| {
				let out = aphrodite::resolve::expand(black_box(&mut state), black_box(root));
				let out = out.expect("chain resolves");
				assert!(out.contains("LEAF CONTENT"), "expansion must reach the leaf");
				out
			});
		});
	}
	g.finish();
}

fn bench_store(c:&mut Criterion) {
	let corpus = load_corpus();
	let picks = ["text_1kb.txt", "text_100kb.txt", "text_500kb.txt"];
	let mut g = c.benchmark_group("store");
	for f in corpus.iter().filter(|f| picks.contains(&f.name.as_str())) {
		let key = headroom_core::ccr::compute_key(f.content.as_bytes());
		g.throughput(Throughput::Bytes(f.content.len() as u64));
		g.bench_with_input(BenchmarkId::new("compute_key", &f.name), &f.content, |b, content| {
			b.iter(|| headroom_core::ccr::compute_key(black_box(content.as_bytes())));
		});
		g.bench_with_input(BenchmarkId::new("put", &f.name), &f.content, |b, content| {
			let mut state = aphrodite::state::AphroditeState::default();
			b.iter(|| state.inline_store_put(key.clone(), content.clone()));
		});
		g.bench_with_input(BenchmarkId::new("get", &f.name), &f.content, |b, content| {
			let mut state = aphrodite::state::AphroditeState::default();
			// 64 entries so `get` pays a realistic scan + LRU promotion cost.
			for i in 0..64 {
				state.inline_store_put(format!("{key}{i:02}"), content.clone());
			}
			state.inline_store_put(key.clone(), content.clone());
			b.iter(|| state.inline_store_get(black_box(&key)).expect("present"));
		});
	}
	g.finish();
}

fn bench_pathological(c:&mut Criterion) {
	// Crash-regression corpus: every stage must complete without panicking.
	// Criterion aborts the run on panic, so simply executing is the assert;
	// the explicit check below also pins that output is produced.
	let mut g = c.benchmark_group("pathological");
	g.sample_size(20);
	for f in pathological_corpus() {
		g.throughput(Throughput::Bytes(f.content.len() as u64));
		g.bench_with_input(BenchmarkId::from_parameter(&f.name), &f.content, |b, content| {
			b.iter(|| {
				let fingerprint = run_all_stages(black_box(content));
				assert!(fingerprint > 0, "stages must produce output, not silently no-op");
				fingerprint
			});
		});
	}
	g.finish();
}

criterion_group!(
	benches,
	bench_classify,
	bench_stage2,
	bench_struct_extract,
	bench_marker,
	bench_resolve,
	bench_store,
	bench_pathological
);
criterion_main!(benches);
