#!/usr/bin/env python3
"""Insert the nightly-rustfmt VSCode override into .vscode/settings.json.

The file uses CRLF line endings; the patch tool cannot match across \r\n.
Insert the rust-analyzer.rustfmt.overrideCommand block (from Current's
settings) right before the "[rust]" section, preserving CRLF endings.
Repo root derived from this script's location.
"""
import pathlib

p = pathlib.Path(__file__).resolve().parents[5] / ".vscode/settings.json"
text = p.read_bytes()

needle = b'"files.trimTrailingWhitespace": true,\r\n  "[rust]": {'
insert = (
    b'"files.trimTrailingWhitespace": true,\r\n'
    b"  // Rust: Aphrodite uses nightly-only rustfmt options (rustfmt.toml:\r\n"
    b"  // space_after_colon = false, imports_granularity, group_imports, ...).\r\n"
    b"  // Stable rustfmt and rust-analyzer's internal formatter silently IGNORE\r\n"
    b"  // those, producing space-after-colon output that diverges from CI\r\n"
    b"  // (Check.yml pins nightly-2026-05-01 for the fmt gate). Force\r\n"
    b"  // rust-analyzer to call the SAME nightly rustfmt as CI so VSCode\r\n"
    b"  // formatting is byte-identical to `cargo fmt --check`.\r\n"
    b'  "rust-analyzer.rustfmt.overrideCommand": [\r\n'
    b'    "rustup",\r\n'
    b'    "run",\r\n'
    b'    "nightly-2026-05-01",\r\n'
    b'    "rustfmt",\r\n'
    b'    "--edition",\r\n'
    b'    "2024"\r\n'
    b"  ],\r\n"
    b'  "[rust]": {'
)

if b"overrideCommand" in text:
    print("already present - no change")
elif needle in text:
    p.write_bytes(text.replace(needle, insert, 1))
    print("inserted nightly rustfmt override")
else:
    print("NEEDLE NOT FOUND - aborting, no change")