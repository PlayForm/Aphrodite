# CEREMONY-AUDIT - Branch-identity audit (Development vs Current)

Date: 2026-09-18. Scope: Aphrodite parent repo + `plugins/aphrodite` submodule.
Method: read-only ref access only - `git fetch Source`, then `git show` /
`git grep` / `git ls-tree` on `Source/Development` and `Source/Current`. **No
checkout of Current, no commits, no fixes** (fixing Current is a separate task).
Trigger: the discovered leak where Current's copy of `Auto.yml` pushes to
`branch: Development`.

## Findings table

| File                                                                                                                                               | Branch             | Verdict                          | Evidence (command + snippet)                                                                                                                                                                                                                                                                                                                                                                                                  |
| -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ | -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.github/workflows/Auto.yml`                                                                                                                       | Current            | **LEAK**                         | `git show Source/Current:.github/workflows/Auto.yml` line 68: push step `branch: Development`. Full quote below.                                                                                                                                                                                                                                                                                                              |
| `.github/workflows/Auto.yml`                                                                                                                       | Development        | clean (sanctioned)               | `git show Source/Development:.github/workflows/Auto.yml` line 68: `branch: Current`.<br>Matches classification `G5-05 +guard` ("pushes Current"; heartbeat touches only the CI-ignored `.github/Update.md` path - accepted exception to I10).                                                                                                                                                                                 |
| `.github/workflows/Check.yml`                                                                                                                      | Development        | clean                            | `git grep -n 'branches:' Source/Development -- .github/workflows` lines 12, 15: `branches: [Development]`.                                                                                                                                                                                                                                                                                                                    |
| `.github/workflows/Check.yml`                                                                                                                      | Current            | clean                            | lines 12, 15: `branches: [Current]`; lines 108-110 comment on Test-job removal = content, not identity.                                                                                                                                                                                                                                                                                                                       |
| `.github/workflows/ffi-check.yml`                                                                                                                  | Development (only) | clean                            | lines 12, 32: `branches: [Development]`; absent from Current by design.                                                                                                                                                                                                                                                                                                                                                       |
| `.github/workflows/Build.yml`, `Publish.yml`, `Dependabot.yml`, `GitHub.yml`                                                                       | both               | clean                            | zero `Current`/`Development` keyword matches on either ref; tag triggers `Aphrodite/v*` are branch-agnostic.                                                                                                                                                                                                                                                                                                                  |
| `.gitmodules`                                                                                                                                      | Development        | clean                            | `git show Source/Development:.gitmodules`: `plugins/aphrodite -> branch = Development`,<br>`vendor/headroom` + `vendor/rtk -> branch = Current` (by design, V7).                                                                                                                                                                                                                                                              |
| `.gitmodules`                                                                                                                                      | Current            | clean                            | `git show Source/Current:.gitmodules`: `plugins/aphrodite -> branch = Current`, vendors `-> branch = Current`.                                                                                                                                                                                                                                                                                                                |
| `plugins/aphrodite` gitlink                                                                                                                        | Development        | clean                            | `git ls-tree Source/Development plugins/aphrodite` = d4b426bb…; `git -C plugins/aphrodite branch --contains d4b426bb…` -> Development.                                                                                                                                                                                                                                                                                        |
| `plugins/aphrodite` gitlink                                                                                                                        | Current            | clean                            | a340e063…; `--contains` -> Current (and remotes/Source/Current).                                                                                                                                                                                                                                                                                                                                                              |
| `.githooks/*`                                                                                                                                      | Current            | LOW (stale comment + divergence) | hooks still tracked on Current although the 2026-09-17 removal removed them repo-wide (ceremony + TAXONOMY line 55: "the hooks are gone").<br>`lib/sync-submodules.sh` lines 8-10 comment describes the Development layout ("Development for plugins/aphrodite on this branch") while Current's own `.gitmodules` says `branch = Current`.<br>Comment-only; the script derives from `.gitmodules` at runtime. Reporting only. |
| `Maintain/scripts/release/auto-release.sh`                                                                                                         | Development        | LOW (guarded)                    | line 27: `RELEASE_BRANCH` derives from HEAD, falls back to `echo Current` on detached HEAD - a detached-HEAD run on Development would release to Current.<br>Parameterized by design (comment lines 25-26); latent risk, not an active leak.                                                                                                                                                                                  |
| `README.md`                                                                                                                                        | both               | clean                            | `tree/Current`/`blob/Current` doc URLs = sanctioned convention (Current is the distributed docs line).<br>Badge drift v2.1.4 (D) vs v2.1.3 (C) = version-track note, not identity.                                                                                                                                                                                                                                            |
| `crates/aphrodite/templates/aphrodite.toml`                                                                                                        | both               | clean                            | no branch keywords; D..C diff = engine-config drift (thresholds, preview templates, directives) only.                                                                                                                                                                                                                                                                                                                         |
| Docs/notes/AGENTS/classification (`TAXONOMY.md`, `CUR-release-infra-identity.md`, `RELEASE-METHODOLOGY.md`, release-notes, `CHANGELOG.md`, skills) | Development        | clean                            | mentions describe the dual-line model (ceremony/taxonomy content),<br>not identity assertions in the wrong branch.                                                                                                                                                                                                                                                                                                            |
| `Maintain/install.sh`                                                                                                                              | Current            | clean                            | `raw.githubusercontent.com/PlayForm/Aphrodite/Current/...` self-reference.                                                                                                                                                                                                                                                                                                                                                    |
| `crates/aphrodite-hermes/Cargo.toml`                                                                                                               | Current            | clean                            | "Currently" inside a comment = false positive.                                                                                                                                                                                                                                                                                                                                                                                |

Verdict legend: **LEAK** = branch-owned identity pointing at the other branch
(ABORT per ceremony I11); clean = matches its branch; LOW = risk/stale note, no
active leak.

## Auto.yml - the offending step (Current's copy)

From `git show Source/Current:.github/workflows/Auto.yml` (push step, lines
65-68):

```yaml
- uses: ad-m/github-push-action@881a6320fdb16eb5318c5054f31c218aec2b324c # v1.3.0
  with:
      github_token: ${{ secrets.GITHUB_TOKEN }}
      branch: Development
```

`branch: Development` on Current's copy means the daily heartbeat commits are
pushed INTO Development (the append-only workshop line), inverting the
branch-owned identity. Development's own copy pushes `branch: Current` (the
sanctioned `G5-05` heartbeat; the touched path `.github/Update.md` is
`paths-ignore`d in Check.yml). The Current copy must push Current; the leak also
matches the user's observation that Current carries Development code. Per
ceremony I11 this is an ABORT-class finding; fix Current separately.

## Ceremony-step diff (RELEASE-METHODOLOGY.md - added, not restructured)

`A0` preconditions (+1 line):

```diff
 [ ] Tags checked free: git tag | grep 'Aphrodite/vX.Y.Z' → empty
+[ ] Branch-identity audit clean on BOTH refs (I11 - see B4)
```

`B0` preconditions (+1 line):

```diff
 [ ] Plugin tree clean (or phantom cleared - see B1 Action 2)
+[ ] Branch-identity audit clean on BOTH refs (I11 - see B4)
```

New `### B4. Branch-identity audit - MANDATORY pre-sync/pre-tag gate (I11)` (+38
lines after `B3`): read-only audit of BOTH refs before any merge/sync/tag

- workflow triggers + push targets (Auto.yml `branch:` must be Current on both
  copies; a Development push target = LEAK), `.gitmodules` branch fields,
  gitlink targets via `branch --contains`, and a `[Development]/[Current]`
  keyword scan; any mismatch = **ABORT the ceremony**, record in
  CEREMONY-AUDIT.md, fix the offending branch separately, re-run clean.

`PART 7` invariant checklist (+3 lines):

```diff
 I10 working copy on Development; Current touched only inside the ritual
+ I11 branch-identity audit: no [Development]/[Current] identity leak on EITHER
+     ref before any merge/sync/tag (workflow triggers + push targets,
+     .gitmodules branch fields, gitlink targets - see B4)
 D1  selective boundary honored (chain-split absent/present per track)
```

`PART 8` cycle (+2 refs): `A0 checks (incl. B4 branch-identity audit)` and
`B4 audit gate re-run (I11)` appended to the PHASE A / PHASE B listings.

## Verification (grep results)

- `git grep -n -E 'branch: (Current|Development)|branches: \[(Current|Development)\]' Source/Development -- .github/workflows`
    - `Source/Development:.github/workflows/Auto.yml:68: branch: Current`
    - `Source/Development:.github/workflows/Check.yml:12,15: branches: [Development]`
    - `Source/Development:.github/workflows/ffi-check.yml:12,32: branches: [Development]`
- same grep on `Source/Current`:
    - `Source/Current:.github/workflows/Auto.yml:68: branch: Development` <-
      LEAK
    - `Source/Current:.github/workflows/Check.yml:12,15: branches: [Current]`
- `.gitmodules` branch fields: D -> `Development` (plugin) / `Current`
  (vendors); C -> `Current` (all three).
- gitlink containment: D gitlink d4b426bb… in submodule `Development`; C gitlink
  a340e063… in submodule `Current`.
- Full-tree keyword scan (all tracked files, both refs): remaining matches are
  doc/taxonomy content or false positives (`Format.sh` `Current=$(pwd)`,
  `Cargo.toml` "Currently"); no other identity leak found.
- `prettier --check` clean on `RELEASE-METHODOLOGY.md` + this report.

## Scope notes

- No branch switched; `Current` NOT fixed (separate task); no commits made.
- Local absolute paths omitted; report is portable.
