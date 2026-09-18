# ffi/ - FFI Pipeline, Codegen & CI

The FFI codegen pipeline (cbindgen -> ctypesgen -> finalize_bindings.py ->
committed `_bindings.py`), its research inputs, the static contract checker,
and the CI fixes that made it green. Root entry point: `../ARCHITECTURE.md`.

| File                                                         | One-line description                                                                                                                                       |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [DEVELOP.md](DEVELOP.md)                                     | Implementation half of the research/develop pair: what was built from each research item, the `free_string` runtime fix, and the full verification matrix. |
| [RESEARCH-UPSTREAM.md](RESEARCH-UPSTREAM.md)                 | Upstream ctypesgen + cbindgen output-shape catalog - the input contract the finalizer rewrites.                                                            |
| [RESEARCH-FORK-INTEGRATION.md](RESEARCH-FORK-INTEGRATION.md) | pypdfium2 ctypesgen fork output shapes, plugin consumer surface, FFI safety contracts, rust-bindgen stress corpus.                                         |
| [CI-FFI-CHECK.md](CI-FFI-CHECK.md)                           | GitHub Actions inspection for the FFI pipeline: run #525 failure breakdown, ffi-check.yml history, CI-vs-local discrepancies, recommendations.             |
| [PAIR-A1.md](PAIR-A1.md)                                     | Pair A1: ruff CI fixes for the generated `_bindings.py` (per-file-ignores) + N812 + codegen hardening.                                                     |
| [PAIR-A2.md](PAIR-A2.md)                                     | Pair A2: stale checker self-test fix (11/12 -> 13/13) + ffi-check.yml trigger-path gap.                                                                    |

See also: `../uml/04-hook-ffi.md` (runtime FFI path), `../uml/09-release-ci.md`
(CI workflows).
