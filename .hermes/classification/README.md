# HPC - Halted-Process Classification

The classification system for the Aphrodite dual-line release flow: every file
in the monorepo is a _halted process_ - opening it resumes it. These documents
classify each file/process by its home phase (Development vs Current), its
kind, its layer, and what the release ceremony (Phase A push-down / Phase B
sync-back) does to it.

## Documents

| Document                                                       | Scope                                                                                                                            | Files classified |
| -------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| [TAXONOMY.md](TAXONOMY.md)                                     | The code grammar `{K}{P}{L}-{N}`, ceremony annotations, commutative-diagram notation, layer map, worked examples, ceremony rules | -                |
| [DEV-engine-build.md](DEV-engine-build.md)                     | Engine + build processes: `crates/**`, root build/config, `vendor/`                                                              | 74               |
| [DEV-knowledge-tests-bench.md](DEV-knowledge-tests-bench.md)   | Dev-side knowledge/test/bench processes: `.hermes/`, `bench/`, `tests/`, `docs/`, `assets/`                                      | 138              |
| [CUR-plugin-loader.md](CUR-plugin-loader.md)                   | Plugin/loader processes: `plugins/aphrodite/**` submodule                                                                        | 15               |
| [CUR-release-infra-identity.md](CUR-release-infra-identity.md) | Release infra + identity processes: `Maintain/`, workflows, hooks, `.gitmodules`, vendor gitlinks                                | 68               |

**Total: 295 files classified.**

## How to read a code

```
{K}{P}{L}-{N}        e.g. MC3-01 +tag+guard
 │ │ │  └ sequence   BINARY_VERSION: Manifest, Current-only, plugin layer,
 │ │ └ layer         seq 01 - bumped LAST in any ceremony (live download
 │ └ phase           pointer), guarded by release ordering.
 └ kind
```

Full grammar in [TAXONOMY.md](TAXONOMY.md#1-code-grammar).
