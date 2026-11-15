//! The disk loader's Issue #6 semantics and the baked-in directive set.
//!
//! `LoadDirectives` is not cwd-relative (the caller passes an explicit path),
//! so these are hermetic tempdir tests - no cwd mutation risk, unlike
//! `ApplyCompression`'s own directory search (04-T7).

use crate::Directive::{Builtin::Fn as LoadedBuiltins, Load::Fn as LoadDirectives, Model::MaximumDirectiveChars};

/// A unique scratch directory per test, auto-removed on drop.
struct TempDirectory(std::path::PathBuf);
impl TempDirectory {
	fn new(Tag:&str) -> Self {
		let path = std::env::temp_dir().join(format!(
			"aphrodite-directives-test-{Tag}-{}",
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_nanos()
		));
		std::fs::create_dir_all(&path).unwrap();
		Self(path)
	}

	fn path(&self) -> std::path::PathBuf { self.0.clone() }
}
impl Drop for TempDirectory {
	fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); }
}

#[test]
fn TestLoadDirectivesReadsMdFilesFromDirectory() {
	let Directory = TempDirectory::new("basic");
	std::fs::write(Directory.path().join("focus.md"), "# focus\nstay concise").unwrap();
	std::fs::write(Directory.path().join("explore.md"), "# explore\nlook around").unwrap();

	let Loaded = LoadDirectives(&Directory.path());
	assert_eq!(Loaded.len(), 2);
	assert_eq!(Loaded["focus"].Content, "# focus\nstay concise");
	assert_eq!(Loaded["explore"].Content, "# explore\nlook around");
}

#[test]
fn TestLoadDirectivesSkipsNonMdFiles() {
	let Directory = TempDirectory::new("skip-non-md");
	std::fs::write(Directory.path().join("focus.md"), "keep me").unwrap();
	std::fs::write(Directory.path().join("README.txt"), "not a directive").unwrap();
	std::fs::write(Directory.path().join("notes"), "no extension at all").unwrap();

	let Loaded = LoadDirectives(&Directory.path());
	assert_eq!(Loaded.len(), 1);
	assert!(Loaded.contains_key("focus"));
}

// ── Issue #6: an existing-but-empty directives directory is an
// INTENTIONAL empty set - empty map, NO built-in fallback (a missing
// directory still falls back to builtins, see
// test_load_directives_missing_dir_returns_builtins). ──
#[test]
fn TestLoadDirectivesExistingEmptyDirectoryReturnsEmptyMap() {
	let Directory = TempDirectory::new("empty-dir");
	// Non-.md files must not count as a usable directive either.
	std::fs::write(Directory.path().join("README.txt"), "not a directive").unwrap();

	let Loaded = LoadDirectives(&Directory.path());
	assert!(
		Loaded.is_empty(),
		"an existing empty directives dir must yield an empty map, not builtins"
	);
}

// ── Issue #6: an empty .md file is a *readable* .md file, so it wins
// over builtins and loads as a directive with empty content. ──
#[test]
fn TestLoadDirectivesEmptyMdFileLoadsEmptyContent() {
	let Directory = TempDirectory::new("empty-md");
	std::fs::write(Directory.path().join("empty.md"), "").unwrap();

	let Loaded = LoadDirectives(&Directory.path());
	assert_eq!(Loaded.len(), 1, "the empty .md file must load, not builtins");
	assert!(Loaded.contains_key("empty"));
	assert_eq!(Loaded["empty"].Content, "");
}

// ── Issue #6: whitespace-only .md content loads verbatim (no trimming
// beyond the size cap). ──
#[test]
fn TestLoadDirectivesWhitespaceOnlyLoadsVerbatim() {
	let Directory = TempDirectory::new("whitespace-md");
	let Ws = "   \n\t\n  \n";
	std::fs::write(Directory.path().join("ws.md"), Ws).unwrap();

	let Loaded = LoadDirectives(&Directory.path());
	assert_eq!(Loaded.len(), 1);
	assert_eq!(Loaded["ws"].Content, Ws, "whitespace-only content must load verbatim");
}

#[test]
fn TestLoadDirectivesMissingDirectoryReturnsBuiltins() {
	let Missing = std::env::temp_dir().join("aphrodite-directives-test-does-not-exist");
	let Loaded = LoadDirectives(&Missing);
	// Built-in directives are now returned as fallback when no
	// `directives/` directory exists on disk.
	assert!(
		!Loaded.is_empty(),
		"missing dir should fall back to baked-in built-in directives"
	);
	assert!(Loaded.contains_key("focus"), "built-in directives must include 'focus'");
}

// ── Built-in directives: baked into the binary via include_str! ──
#[test]
fn TestLoadedBuiltinsContainsAllSeven() {
	let Builtins = LoadedBuiltins();
	assert_eq!(Builtins.len(), 7, "should have 7 baked-in directives");
	assert!(Builtins.contains_key("focus"));
	assert!(Builtins.contains_key("foresight"));
	assert!(Builtins.contains_key("ccr-handling"));
	assert!(Builtins.contains_key("cleanup"));
	assert!(Builtins.contains_key("explore"));
	assert!(Builtins.contains_key("lazy"));
	assert!(Builtins.contains_key("lazy-eval"));
}

#[test]
fn TestLoadDirectivesTruncatesAtMaximumCharsWithEllipsis() {
	let Directory = TempDirectory::new("truncate");
	let Oversized = "x".repeat(MaximumDirectiveChars + 500);
	std::fs::write(Directory.path().join("huge.md"), &Oversized).unwrap();

	let Loaded = LoadDirectives(&Directory.path());
	let Content = &Loaded["huge"].Content;
	// MAX_DIRECTIVE_CHARS worth of 'x' plus the ellipsis marker.
	assert_eq!(Content.chars().count(), MaximumDirectiveChars + 3);
	assert!(Content.ends_with("..."));
}

#[test]
fn TestLoadDirectivesUnderCapIsNotTruncated() {
	let Directory = TempDirectory::new("under-cap");
	let Small = "short directive body";
	std::fs::write(Directory.path().join("small.md"), Small).unwrap();

	let Loaded = LoadDirectives(&Directory.path());
	assert_eq!(Loaded["small"].Content, Small);
	assert!(!Loaded["small"].Content.ends_with("..."));
}
