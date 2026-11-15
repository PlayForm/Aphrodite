# CI parity: linting the Linux target from macOS

`.github/workflows/Check.yml` runs on Linux. A gate that is green on macOS proves
nothing about it, because every `#[cfg]`-gated name can be used on one side only.

## The failure class

`unused_variables` fires on Linux for a parameter used solely inside
`#[cfg(target_os = "macos")]`, and `-D warnings` makes it fatal. The fix is to
make the name earn its place on Linux - `Setup/Dylib/Copy/Install/Fn.rs` puts the
artifact name into the non-macOS branch's context message - never to rename it
`_Name`, which keeps a parameter the Linux path does not need.

Sweep the class, not the instance:

```sh
grep -rn 'cfg(target_os' crates/*/Source --include=*.rs
```

Each site is either a per-target `const`/`mod` definition (no names to check) or a
gated body whose names must be used on both sides. `cfg(windows)`, `cfg(unix)` and
`cfg(feature)` gates have the same shape: a name bound inside the gate and used
after it is fine, a name used only inside the gate is the bug.

## Running the Linux clippy

`x86_64-unknown-linux-gnu` installs but cannot build `ring`: there is no
`x86_64-linux-gnu-gcc` on this machine, and clippy still runs dependency build
scripts (`error occurred in cc-rs: command did not execute successfully`).
`x86_64-unknown-linux-musl` is a Linux target (`target_os = "linux"`, the only cfg
that matters here) and works with Homebrew `musl-cross`:

```sh
rustup target add x86_64-unknown-linux-musl
export CC_x86_64_unknown_linux_musl=x86_64-linux-musl-gcc \
	AR_x86_64_unknown_linux_musl=x86_64-linux-musl-ar \
	CXX_x86_64_unknown_linux_musl=<shim> \
	CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-linux-musl-gcc \
	CFLAGS_x86_64_unknown_linux_musl="-O0"
cargo clippy --target x86_64-unknown-linux-musl -p aphrodite -p aphrodite-hermes -- -D warnings; echo "EXIT:$?"
```

The `musl-cross` prefix is `/opt/homebrew/opt/musl-cross`. `/usr/local/opt/musl-cross`
does not exist, and a wrong prefix there surfaces as `sccache: Compiler not
supported ... No such file or directory`, not as a missing-toolchain error.

## The esaxx-rs shim

`esaxx-rs`'s build script picks `-stdlib=libc++` from `cfg!(target_os = "macos")`,
and inside a build script that macro means the HOST. Cross-compiling from macOS
therefore feeds a libc++ flag to a Linux g++, which fails with `unrecognized
command-line option '-stdlib=libc++'`. `<shim>` is a script that drops every
`-stdlib=*` argument and execs the real compiler:

```bash
#!/bin/bash
out=()
for a in "$@"; do
	case "$a" in
		-stdlib=*) ;;
		*) out+=("$a") ;;
	esac
done
exec /opt/homebrew/opt/musl-cross/bin/x86_64-linux-musl-g++ "${out[@]}"
```

## Reading the result

`dropping unsupported crate type 'cdylib' for target 'x86_64-unknown-linux-musl'`
is expected: linting needs no cdylib, and the lib is still linted as an rlib. Any
`error:` line is a real failure, since `-D warnings` already promoted warnings to
it.

A clippy fix is not verified until this exit code is 0. The macOS run is the
baseline, never the evidence.
