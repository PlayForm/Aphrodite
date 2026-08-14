//! Packaging guard.
//!
//! `cargo publish` never checks that the files referenced by `include_str!`
//! actually ship in the uploaded tarball - it only validates the manifest
//! (that a lib/bin target exists). The v1.3.8 release was un-installable
//! because `exclude = ["*.md"]` stripped `src/builtin_directives/*.md`, which
//! `src/directives.rs` embeds via `include_str!`. Local `cargo build`/`cargo
//! test` passed because the files exist in the checkout; only a build of the
//! *packaged tarball* (what `cargo install` consumes) reveals the break.
//!
//! This test shells out to `cargo package --list` (the real tarball contents)
//! and asserts every `include_str!("src/...")` target in `src/directives.rs`
//! is present. It runs in `cargo test`, so CI's `Test` job catches regressions
//! before any publish. A `build.rs` cannot do this reliably: it runs against
//! the checkout (files present) and is skipped under `cargo publish --no-verify`.

use std::path::Path;
use std::process::Command;

/// Extract the path argument from `include_str!("...")` occurrences wherever
/// they appear on a line (they sit inside a tuple, e.g.
/// `("focus", include_str!("builtin_directives/focus.md"))`). The crate embeds
/// files relative to `src/`, so in the tarball they appear as
/// `src/builtin_directives/focus.md`. We normalize both `src/...` and
/// `builtin_directives/...` forms to the tarball path.
fn collect_include_str_paths(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        let mut rest = line;
        // find every `include_str!("...")` on the line
        while let Some(pos) = rest.find("include_str!") {
            let after_kw = &rest[pos + "include_str!".len()..];
            if let Some(open) = after_kw.find('"') {
                let inner = &after_kw[open + 1..];
                if let Some(close) = inner.find('"') {
                    let path = &inner[..close];
                    let normalized = path
                        .strip_prefix("src/")
                        .unwrap_or(path);
                    if normalized.starts_with("builtin_directives/") {
                        out.push(format!("src/{}", normalized));
                    }
                    rest = &inner[close + 1..];
                    continue;
                }
            }
            break;
        }
    }
    out
}

#[test]
fn packaged_tarball_contains_all_builtin_directives() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");

    // 1. Read directives.rs and find its include_str! targets.
    let directives_path = Path::new(crate_dir).join("src/directives.rs");
    let source = std::fs::read_to_string(&directives_path)
        .expect("src/directives.rs must be readable in the checkout");
    let targets = collect_include_str_paths(&source);
    assert!(
        !targets.is_empty(),
        "expected at least one include_str! target in src/directives.rs"
    );

    // 2. List the real packaged tarball contents.
    let output = Command::new("cargo")
        .args(["package", "--list", "--allow-dirty"])
        .current_dir(crate_dir)
        .output()
        .expect("failed to spawn `cargo package --list`");
    assert!(
        output.status.success(),
        "cargo package --list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let listed = String::from_utf8_lossy(&output.stdout);

    // 3. Every include_str! target must appear in the tarball listing.
    for target in &targets {
        assert!(
            listed.lines().any(|l| l.trim_end() == target.as_str()),
            "packaged tarball is missing `{}` (referenced by include_str! in src/directives.rs).
\
             Fix: ensure it is not excluded by `exclude` in Cargo.toml.",
            target
        );
    }
}

#[test]
fn packaged_tarball_has_lib_and_bin_targets() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let output = Command::new("cargo")
        .args(["package", "--list", "--allow-dirty"])
        .current_dir(crate_dir)
        .output()
        .expect("failed to spawn `cargo package --list`");
    assert!(output.status.success());
    let listed = String::from_utf8_lossy(&output.stdout);
    assert!(
        listed.lines().any(|l| l.trim_end() == "src/lib.rs"),
        "packaged tarball missing src/lib.rs - no [lib] target would ship"
    );
    assert!(
        listed.lines().any(|l| l.trim_end() == "src/main.rs"),
        "packaged tarball missing src/main.rs - no [[bin]] target would ship"
    );
}
