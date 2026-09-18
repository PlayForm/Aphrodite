# release/ - Release Ceremony, Methodology & Sync-Back

The dual-line release system: Development (workshop) pushes down to Current
(distributed) in Phase A, Current syncs back selectively in Phase B, with the
B4 branch-identity audit gate. Root entry point: `../CEREMONY.md`.

| File                                                               | One-line description                                                                                                                                    |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [RELEASE-METHODOLOGY.md](RELEASE-METHODOLOGY.md)                   | THE canonical operational spec: every push/pull/tag/release step action-by-action (Phase A, Phase B, B4 gate, tagging, formatter contract, invariants). |
| [RELEASE-STRATEGY-1.4.3-1.5.0.md](RELEASE-STRATEGY-1.4.3-1.5.0.md) | Dual-track release strategy design: selective-promotion boundary, tag mechanics, git-graph shape for v1.4.3 + v1.5.0.                                   |
| [CEREMONY-AUDIT.md](CEREMONY-AUDIT.md)                             | Branch-identity audit findings: Current's `Auto.yml` pushes to `branch: Development` (ABORT-class leak), B4 gate added to the methodology.              |
| [DUAL-STATE-MERGE-REVIEW.md](DUAL-STATE-MERGE-REVIEW.md)           | Dual-state merge review method (staged proposal / pending correction) for reverse-direction merges; worked example: Current → Development sync-back.   |
| [MERGE-SUBMODULE-EXEC.md](MERGE-SUBMODULE-EXEC.md)                 | Phase B plugin sync-back execution record: cherry-pick `f8f4cdf` → `098134e` (`-x` trailer, phantom gitlink hunk dropped).                             |
| [MERGE-SUBMODULE-AUDIT.md](MERGE-SUBMODULE-AUDIT.md)               | Independent auditor report of the submodule sync-back pick: SHA-256 pre/post captures, 7-check PASS table, NOTHING LOST.                              |
| [MERGE-PARENT-EXEC.md](MERGE-PARENT-EXEC.md)                       | Phase B sync-back execution (parent): empty cherry-pick set (all real fixes already in Development), B2 gitlink bump to `098134e` landed in `a613948`.  |
| [MERGE-PARENT-AUDIT.md](MERGE-PARENT-AUDIT.md)                     | Independent auditor report of the parent sync-back: empty pick set content-verified, protected paths unchanged, NOTHING LOST.                         |

Archived: `MERGE-SUBMODULE.md` (pre-finalization draft of the same pick,
intermediate `5fde801` hash) → `../ops/ARCHIVE-2026-09-18/MERGE-SUBMODULE.md`.

The B4 branch-identity audit and the open Auto.yml leak are rolled up in
`../CEREMONY.md` and `../HANDOFF.md`.
