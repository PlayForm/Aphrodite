# .hermes/notes/ - Durable Dev Archive

The Aphrodite dev archive, organized as a **reverse hierarchical taxonomy**
(coarse-to-fine directories, like a library). Every `.md` here is a durable
record; nothing was deleted during the 2026-09-18 rewrite - per-task reports
were moved into category directories, each with an INDEX.md.

## Reading order

| Doc                                                          | What it is                                                                                                                 |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- |
| [ARCHITECTURE.md](ARCHITECTURE.md)                           | The current-state architecture: FFI pipeline, runtime home, plugin loader, preview system.                                 |
| [ISSUE-11.md](ISSUE-11.md)                                   | The full Issue #11 history: root cause, fix design, what landed (WS1/2/4 + residuals), battery before/after, what remains. |
| [CEREMONY.md](CEREMONY.md)                                   | The release ceremony: Phase A push-down, Phase B sync-back, B4 branch-identity audit gate.                                 |
| [SESSION-LOG-2026-09-17-18.md](SESSION-LOG-2026-09-17-18.md) | The chronological session record (2026-09-15 -> 2026-09-18), absorbing every per-task report as a dated entry.             |
| [HANDOFF.md](HANDOFF.md)                                     | The current pending queue - read this first when resuming work.                                                            |

## The taxonomy

| Category                     | Scope                                                                                      | Index    |
| ---------------------------- | ------------------------------------------------------------------------------------------ | -------- |
| [ffi/](ffi/INDEX.md)         | FFI pipeline, codegen, research, hardening, FFI CI                                         | 6 files  |
| [issue11/](issue11/INDEX.md) | Issue #11 preview-collapse bug family: root cause, fix design, batteries, landed, residual | 13 files |
| [release/](release/INDEX.md) | Release ceremony, methodology, sync-back, CI triggers, merge review method                 | 8 files  |
| [session/](session/INDEX.md) | Chronological session continuations, handoffs, dispatches                                  | 4 files  |
| [plugin/](plugin/INDEX.md)   | Plugin loader, layout self-heal, directives, failure forensics                             | 5 files  |
| [ops/](ops/INDEX.md)         | Operational + meta notes (docs refresh, benchmarks, optimization clues, rewrite reports)   | 10 files |

Plus the rewrite-session reports at this root: `HERMES-REWRITE-A.md` (this
taxonomy + uml + classification pass) and `HERMES-REWRITE-B.md` (the AGENTS.md

- skills + tmp tidy pass). The 1.5.0 preview-refactor blueprint lives at
  `ops/SPLIT-ADAPTATION.md` (research distillation) with the concrete plan at
  `ops/REFACTOR-PLAN-1.5.0.md`.

Format contract: prettier-clean (repo `.prettierrc`: tabs, width 100,
proseWrap preserve), anonymized (zero local absolute paths - repo-relative or
`…/`-elided references only).
