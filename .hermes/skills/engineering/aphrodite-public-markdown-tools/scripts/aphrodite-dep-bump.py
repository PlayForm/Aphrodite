#!/usr/bin/env python3
"""Bump crates/aphrodite/Cargo.toml direct deps to Current's pins
(= latest crates.io, verified 2026-09-18). Literal replaces, each asserted
to match exactly once. Leaves the crate version (1.4.6) untouched.

Repo root is derived from this script's location, so it works from any
clone of the repo (no hardcoded <WORKSPACE> paths).
"""

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[5]
PATH = REPO / "crates/aphrodite/Cargo.toml"

# (old, new) - Development -> Current (= crates.io latest)
BUMPS = [
    ('anyhow = { version = "1.0.102", optional = true }',
     'anyhow = { version = "1.0.104", optional = true }'),
    ('blake3 = "1.7"', 'blake3 = "1.8"'),
    ('clap = { version = "4.6.1", features = ["derive", "env"], optional = true }',
     'clap = { version = "4.6.7", features = ["derive", "env"], optional = true }'),
    ('futures = { version = "0.3.32", optional = true }',
     'futures = { version = "0.3.34", optional = true }'),
    ('lru = { version = "0.18.0", optional = true }',
     'lru = { version = "0.18.4", optional = true }'),
    ('rand = { version = "0.10.1", optional = true }',
     'rand = { version = "0.10.2", optional = true }'),
    ('regex = "1.12.4"', 'regex = "1.13.1"'),
    ('reqwest = { version = "0.13.4", default-features = false, features = [',
     'reqwest = { version = "0.13.5", default-features = false, features = ['),
    ('tokio = { version = "1.52.3", features = [',
     'tokio = { version = "1.53.1", features = ['),
    ('tokio-util = { version = "0.7.18", features = ["rt"], optional = true }',
     'tokio-util = { version = "0.7.19", features = ["rt"], optional = true }'),
    ('toml = "1.1"', 'toml = "1.1.6+spec-1.1.0"'),
    ('uuid = { version = "1.23.3", features = ["v4"], optional = true }',
     'uuid = { version = "1.26.1", features = ["v4"], optional = true }'),
]

with open(PATH, encoding="utf-8") as f:
    text = f.read()

for old, new in BUMPS:
    hits = text.count(old)
    if hits != 1:
        print(f"REFUSE: {old!r} matched {hits} times (expected 1); file untouched")
        sys.exit(1)
    text = text.replace(old, new)

with open(PATH, "w", encoding="utf-8", newline="") as f:
    f.write(text)

print(f"OK: {len(BUMPS)} dep bumps applied")