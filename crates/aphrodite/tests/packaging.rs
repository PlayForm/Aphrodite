//! Packaging guard.
//!
//! `cargo publish` never checks that the files referenced by `include_str!`
//! actually ship in the uploaded tarball - it only validates the manifest
//! (that a lib/bin target exists). The v1.3.8 release was un-installable
//! because `exclude = ["*.md"]` stripped the builtin directive `.md` files,
//! which `Source/Directive/Builtin.rs` embeds via `include_str!`. Local
//! `cargo build`/`cargo test` passed because the files exist in the checkout;
//! only a build of the *packaged tarball* (what `cargo install` consumes)
//! reveals the break.
//!
//! This test shells out to `cargo package --list` (the real tarball contents)
//! and asserts every `include_str!` target in `Source/Directive/Builtin.rs` is
//! present. It runs in `cargo test`, so CI's `Test` job catches regressions
//! before any publish. A `build.rs` cannot do this reliably: it runs against
//! the checkout (files present) and is skipped under `cargo publish --no-verify`.

use std::{path::Path, process::Command};

/// Extract the path argument from `include_str!("...")` occurrences wherever
/// they appear on a line (they sit inside a tuple, e.g.
/// `("focus", include_str!("../BuiltinDirective/focus.md"))`). The path is
/// relative to the embedding file (`Source/Directive/`), so in the tarball the
/// target appears as `Source/BuiltinDirective/focus.md`. We normalize the
/// `../` and `Source/` forms to that tarball path.
fn CollectIncludeStrPaths(Source:&str) -> Vec<String> {
	let mut Out = Vec::new();
	for Line in Source.lines() {
		let Line = Line.trim();
		let mut Rest = Line;
		// find every `include_str!("...")` on the line
		while let Some(Pos) = Rest.find("include_str!") {
			let AfterKw = &Rest[Pos + "include_str!".len()..];
			if let Some(Open) = AfterKw.find('"') {
				let Inner = &AfterKw[Open + 1..];
				if let Some(Close) = Inner.find('"') {
					let path = &Inner[..Close];
					let Normalized = path.trim_start_matches("../");
					let Normalized = Normalized.strip_prefix("Source/").unwrap_or(Normalized);
					if Normalized.starts_with("BuiltinDirective/") {
						Out.push(format!("Source/{}", Normalized));
					}
					Rest = &Inner[Close + 1..];
					continue;
				}
			}
			break;
		}
	}
	Out
}

#[test]
fn PackagedTarballContainsAllBuiltinDirectives() {
	let CrateDirectory = env!("CARGO_MANIFEST_DIR");

	// 1. Read Builtin.rs (where the include_str! targets live) and find them.
	let DirectivesPath = Path::new(CrateDirectory).join("Source/Directive/Embedded/Const.rs");
	let Source = std::fs::read_to_string(&DirectivesPath)
		.expect("Source/Directive/Embedded/Const.rs must be readable in the checkout");
	let Targets = CollectIncludeStrPaths(&Source);
	assert!(
		!Targets.is_empty(),
		"expected at least one include_str! target in Source/Directive/Embedded/Const.rs"
	);

	// 2. List the real packaged tarball contents.
	let Output = Command::new("cargo")
		.args(["package", "--list", "--allow-dirty"])
		.current_dir(CrateDirectory)
		.output()
		.expect("failed to spawn `cargo package --list`");
	assert!(
		Output.status.success(),
		"cargo package --list failed: {}",
		String::from_utf8_lossy(&Output.stderr)
	);
	let Listed = String::from_utf8_lossy(&Output.stdout);

	// 3. Every include_str! target must appear in the tarball listing.
	for Target in &Targets {
		assert!(
			Listed.lines().any(|l| l.trim_end() == Target.as_str()),
			"packaged tarball is missing `{}` (referenced by include_str! in Source/BuiltinDirective/).
Fix: ensure it is not excluded by `exclude` in Cargo.toml.",
			Target
		);
	}
}

#[test]
fn PackagedTarballHasLibAndBinTargets() {
	let CrateDirectory = env!("CARGO_MANIFEST_DIR");
	let Output = Command::new("cargo")
		.args(["package", "--list", "--allow-dirty"])
		.current_dir(CrateDirectory)
		.output()
		.expect("failed to spawn `cargo package --list`");
	assert!(Output.status.success());
	let Listed = String::from_utf8_lossy(&Output.stdout);
	assert!(
		Listed.lines().any(|l| l.trim_end() == "Source/Library.rs"),
		"packaged tarball missing Source/Library.rs - no [lib] target would ship"
	);
	assert!(
		Listed.lines().any(|l| l.trim_end() == "Source/Main.rs"),
		"packaged tarball missing Source/Main.rs - no [[bin]] target would ship"
	);
}
