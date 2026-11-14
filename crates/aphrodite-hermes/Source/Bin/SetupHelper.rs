//! Install-helper binary for `cargo install aphrodite-hermes`.
//!
//! This crate also builds a cdylib (`libaphrodite_hermes.{dylib,so,dll}`)
//! consumed by the Hermes Python plugin via ctypes - but `cargo install`
//! only ever copies a crate's `[[bin]]` target into `~/.cargo/bin/`; it
//! never distributes library-crate-type outputs (rlib/cdylib) anywhere,
//! and there is no cargo mechanism that makes this binary able to locate
//! or relocate that artifact after the fact (`CARGO_MANIFEST_DIR` is a
//! *build-time* constant pointing at wherever cargo happened to check out
//! this crate's source to compile it - for a real `cargo install
//! aphrodite-hermes` from crates.io that is an ephemeral registry-cache
//! path, not a stable location, and it does not contain the built dylib
//! either way; the dylib lives under that checkout's `target/release/`,
//! which cargo's own install machinery may already have cleaned up by the
//! time this binary ever runs). A previous version of this binary printed
//! a specific dylib path built from `CARGO_MANIFEST_DIR` - that path was
//! wrong in every real case; removed rather than left misleading.
//!
//! So this binary exists only to confirm the crate installed and to point
//! the user at how to actually get the dylib: `aphrodite setup` (which
//! fails loudly and specifically if it can't find one) or a full source
//! build (`cargo build --release -p aphrodite-hermes`).

fn main() {
	println!("aphrodite-hermes v{} installed.", env!("CARGO_PKG_VERSION"));
	println!(
		"`cargo install` does not distribute this crate's dylib (libaphrodite_hermes.*) - only this helper binary."
	);
	println!("Run 'aphrodite setup' next: it will report exactly what it can't find, if anything.");
	println!("If it can't find the dylib, build it from a full source checkout:");
	println!("  cargo build --release -p aphrodite-hermes");
}
