# HPC Classification - Taxonomy

**Purpose:** every file in the Aphrodite monorepo is a _halted process_ - opening
it resumes it. This taxonomy classifies each file/process by its home phase
(Development vs Current), its kind, its layer, and what the release ceremony
(Phase A push-down / Phase B sync-back) does to it, so the dual-line flow can
be reasoned about like a commutative diagram: _which process feeds which
process, and where it lands in the Tag_.

Status: **v0.1 - working taxonomy**. The four classification passes in this
directory apply it file-by-file; amendments live below in the amendment log.

---

## 1. Code grammar

```
{K}{P}{L}-{N}[annotation...]
```

| Slot        | Values                                                                                                                               | Meaning                                   |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------- |
| `K` (kind)  | `P` Process · `A` Artifact · `C` Config · `T` Test · `D` Doc · `S` Script · `M` Manifest · `G` Git-meta · `V` Vendor                 | what the halted process is                |
| `P` (phase) | `D` Development · `C` Current · `B` Both (shared) · `R` Ritual (ceremony-time only)                                                  | which branch line the process is alive on |
| `L` (layer) | `1` core engine · `2` hermes bridge · `3` plugin/loader · `4` release infra · `5` identity/protected · `6` meta/process · `7` vendor | which subsystem it belongs to             |
| `N` (seq)   | 01…99                                                                                                                                | sequence within (kind, phase, layer)      |

### Ceremony annotations (suffix, comma-separated)

| Annotation  | Meaning                                                                                                                   |
| ----------- | ------------------------------------------------------------------------------------------------------------------------- |
| `→C` / `→D` | ceremony destination: what the process becomes during Phase A (push-down to Current) / Phase B (sync-back to Development) |
| `+tag`      | participates in the Tag - receives special attention during the release ceremony                                          |
| `+bump`     | version-bumped during the ceremony                                                                                        |
| `+float`    | gitlink floated to a specific commit during the ceremony                                                                  |
| `+guard`    | hook-guarded (pre-commit branch guard, submodule-pin, auto-bump)                                                          |
| `∅`         | NEVER crosses the phase boundary (identity)                                                                               |
| `@R`        | only exists during the ceremony (ritual process)                                                                          |

### Commutative-diagram notation

```
D:{file}  --Phase A-->  C:{file'}
C:{file}  --Phase B-->  D:{file''}
```

Reads as: _this process, while alive on Development, is pushed down to Current
by Phase A; that Current process is pulled back up by Phase B._

---

## 2. Phase semantics (the dual-line model)

- **Development (workshop):** accumulates work, runs ALL tests + CI, holds
  dev scaffolding (`.hermes/`, `bench/`, `tests/`, skills, profiles history),
  append-only, never rebased, NO release tags.
- **Current (distributed):** test-free line, tags + GitHub releases live ONLY
  here, content arrives by snapshot transplant (Phase A), hotfixes worked
  directly here then picked up by `cherry-pick -x` (Phase B).
- **Both (shared):** code, config, docs that transfer in both directions -
  each direction is a _selective pick_, never an automatic merge.
- **Ritual (ceremony-time only):** release notes staging, tag objects,
  squash-staging area, the ceremony window itself.

---

## 3. Layer map

| Layer                | Where                                                   | Examples                                                                           |
| -------------------- | ------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| 1 core engine        | `crates/aphrodite/`                                     | engine, config loader, directives builtins, proxy                                  |
| 2 hermes bridge      | `crates/aphrodite-hermes/`                              | dylib FFI, schemas, tools, hooks, materialize                                      |
| 3 plugin/loader      | `plugins/aphrodite/`                                    | `__init__.py` shim, plugin.yaml, download.sh/ps1, BINARY_VERSION, layout self-heal |
| 4 release infra      | `Maintain/`, `.github/workflows/`, release scripts      | release notes, CI triggers, build/publish                                          |
| 5 identity/protected | `.gitmodules`, gitlink, `.githooks/`, workflow triggers | branch-owned, never cross                                                          |
| 6 meta/process       | `.hermes/`, `docs/`, skills, plans                      | knowledge + methodology processes                                                  |
| 7 vendor             | `vendor/headroom`, `vendor/rtk`                         | third-party, pinned                                                                |

---

## 4. Worked examples

| File                                     | Code                | Reasoning                                                                                                                                         |
| ---------------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `plugins/aphrodite/BINARY_VERSION`       | `MC3-01 +tag+guard` | Manifest, Current-only (pairs the plugin with a binary release), bumped LAST in any ceremony (live download pointer), guarded by release ordering |
| `crates/aphrodite/src/lib.rs`            | `PD1-01 →C +bump`   | Process, Development home, ships via Phase A, version rides the release                                                                           |
| `plugins/aphrodite/__init__.py`          | `PB3-01 →C +tag`    | Loader process, both phases, must stay byte-identical to `crates/aphrodite/templates/__init__.py` (setup.rs drift guard)                          |
| `crates/aphrodite/templates/__init__.py` | `MB1-05 +guard`     | Manifest-embedded copy, both phases, drift-guard test keeps it identical to the plugin                                                            |
| `.gitmodules`                            | `G5-01 ∅`           | Identity - never crosses; branch field is per-branch                                                                                              |
| `.hermes/notes/RELEASE-METHODOLOGY.md`   | `DD6-01 ∅`          | Doc, Development-only, never ships                                                                                                                |
| `Maintain/release-notes-vX.Y.Z.md`       | `D@R4-01 →C +tag`   | Ritual doc - staged during ceremony, feeds the GitHub release notes                                                                               |
| `.github/workflows/Check.yml`            | `G5-02 ∅`           | Trigger identity - `[Development]` vs `[Current]` variant per branch                                                                              |
| `plugins/aphrodite/layout_check.py`      | `PD3-02 →C`         | Self-heal process, Development home, ships (runtime home is on the user side)                                                                     |
| `vendor/headroom/**`                     | `V7-01`             | Pinned third-party                                                                                                                                |

---

## 5. Ceremony rules encoded

1. **Submodule first, always:** `plugins/aphrodite` (layer 3) syncs before the
   parent in both Phase A and Phase B - the parent gitlink must reference the
   plugin's tip (bottom-up).
2. **BINARY_VERSION bumped LAST** (`MC3-01 +tag`): it is a LIVE distribution
   pointer; bumping it before the release assets exist breaks every download.
3. **Identity never crosses** (`G5-* ∅`): `.gitmodules`, workflow triggers,
   gitlink are restored branch-owned at every transplant (Action 10/7).
4. **`+tag` processes** are the Tag composition: version bumps (`+bump`),
   `BINARY_VERSION`, release notes, gitlink float, tag object.
5. **Dev scaffolding stays** (`D*6`/`T*`/`bench`): tests, `.hermes/`, bench
   never reach Current; Development KEEPS tests in Phase B.
6. **Template drift guard** (`MB1-05`): `templates/__init__.py` byte-identical
   to the live plugin shim - formatter contract, setup.rs asserts it.

---

## 6. Amendment log

| Date       | Amendment                                                                                                                                                 |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2026-09-17 | v0.1 taxonomy established; four classification passes dispatched (DEV crates/build, DEV tests/bench/docs, CUR plugin/loader, CUR release-infra/identity). |

## 7. Verification

- **295 files classified** across the four passes (74 + 138 + 15 + 68).
- All pass files share one table schema: `Path | HPC code | Phase | Ceremony
behavior | Halted-process resume note`.
- `npx prettier --check .hermes/classification/*.md` passes (repo md style:
  tabs, width 100, proseWrap preserve).
- The four pass files each carry their own count verification in section 5.
