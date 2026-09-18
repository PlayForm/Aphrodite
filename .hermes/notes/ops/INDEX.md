# ops/ - Operational & Meta Notes

Miscellaneous operational notes: the docs-refresh record, benchmarking
comparisons, optimization clues, and the meta-reports about rewriting the
notes themselves (kept for provenance).

| File                                             | One-line description                                                                                                                                                                                                        |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [DOCS-UPDATE.md](DOCS-UPDATE.md)                 | The `.hermes/uml/` + AGENTS.md refresh record (v1.4.6 / 2.1.4): what changed per file, stale-term grep, source-verified facts.                                                                                              |
| [COMPARISON.md](COMPARISON.md)                   | Headroom passthrough vs Aphrodite CCR: same 30-turn session, 150 tool calls - 47% less context fill, ~100x tool compression.                                                                                                |
| [optimization-clues.md](optimization-clues.md)   | Scannable perf clue index for a later optimization pass (v1.3.4-era; several clues landed in 1.4.6 via `adf7f36`).                                                                                                          |
| [REWRITE-A.md](REWRITE-A.md)                     | Wide-table expansion report, partner A (8 files): format applied, content-preservation proof, verification.                                                                                                                 |
| [REWRITE-B.md](REWRITE-B.md)                     | Wide-table expansion report, partner B (5 files): per-file summary, backtick balance, prettier result.                                                                                                                      |
| [TABLES-A.md](TABLES-A.md)                       | Wide-table restore report, partner A (4 files): original table format restored from git history.                                                                                                                            |
| [TABLES-B.md](TABLES-B.md)                       | Wide-table restore report, partner B (2 files): original table format restored from git history.                                                                                                                            |
| [REFACTOR-PLAN-1.5.0.md](REFACTOR-PLAN-1.5.0.md) | 1.5.0 preview/detection-layer refactor plan: declarative detector pipeline, typed `Input`, `Option::or_else` dispatcher, regex elimination, reverse-taxonomy file tree, 7-phase migration, risk register.                   |
| [SPLIT-ADAPTATION.md](SPLIT-ADAPTATION.md)       | split.md research distillation: regex-crate per-dimension winners + no-new-deps verdict, declarative detector-pipeline blueprint, reverse-taxonomy principle, session-sequencing practice. Original adaptation.             |
| [FORMAT-ALIGN.md](FORMAT-ALIGN.md)               | Byte-identical formatting Development ↔ Current (2026-09-18): nightly rustfmt + ruff format + prettier run, 15-pair byte-identity proof, rustfmt.toml templates-ignore port, `_bindings.py` restore + extend-exclude guard. |
