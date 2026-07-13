//! Install-helper binary for `cargo install aphrodite-hermes`.
//! This crate is a cdylib consumed by the Hermes plugin; this binary
//! exists solely so `cargo install` can place the .dylib alongside it
//! in ~/.cargo/bin/ where `aphrodite setup` will find it.

fn main() {
	println!(
		"aphrodite-hermes v{} — dylib installed for Hermes plugin.",
		env!("CARGO_PKG_VERSION")
	);
	#[cfg(target_os = "macos")]
	eprintln!("dylib: {}/libaphrodite_hermes.dylib", env!("CARGO_MANIFEST_DIR"));
	println!("Run 'aphrodite setup' to complete the installation.");
}
