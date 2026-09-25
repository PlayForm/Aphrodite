# Formatter parity between editor and CI (byte-identical mirrors)

Reference for the mirror-discipline rule in SKILL.md. Each item is a worked
instance of a rule; the rule itself lives in SKILL.md. All commands are
byte-stable.

## Nightly-only rustfmt options drift the CI gate

A repo whose rustfmt config uses nightly-only options (`space_after_colon =
false`, `imports_granularity`) formats DIFFERENTLY under VSCode's default
formatter (rust-analyzer / stable rustfmt silently ignore unstable options),
so the CI fmt gate fails on every push while the code looks fine locally.

Point the editor at the exact CI toolchain via `.vscode/settings.json`
`rust-analyzer.rustfmt.overrideCommand`:

```sh
rustup run nightly-<CI-pin> rustfmt --edition <Y>
```

Bind Python to the repo's ruff with explicit `ruff.toml` settings.

A submodule with no config of its own silently falls back to defaults
(88-char ruff, stable rustfmt) while the parent uses 100 - that asymmetry is
what drifts byte-identical mirrors.

## Mirror discipline

A file that must stay byte-equal to a source elsewhere (an embedded
template/shim asserted by a test, a copy carried into the build) must be
excluded from EVERY formatter (`rustfmt.toml` `ignore` + `ruff.toml`
`extend-exclude`), or any formatter pass drifts it and the assertion fails.

To reformat the pair: format the SOURCE first, then copy it over the mirror
- never format the mirror alone, and never run `cargo fmt --all` blindly
over a tree containing such a pair. After any formatter pass, re-verify
byte-equality and re-run the asserting test.