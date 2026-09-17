# Aphrodite Release Methodology - The Full Two-Phase Process

**Status: canonical operational spec.** Every push, pull, tag, and release step,
action-by-action, for the dual-line model. Read this before ANY release work.
The two lines are **Development** (work/workshop, append-only, all CI+tests) and
**Current** (release/distributed, tags + GitHub releases ONLY here, test-free).

---

## PART 0 - The mental model (read once, internalize)

```
Development (workshop)          Current (distributed/release)
  work, bump, prep               users download THIS
  ALL tests + CI here            NO tests, NO bench, NO .hermes (incl. skills)
  append-only, never rebased     tags + releases live ONLY here
  no release tags                snapshot transplants, not merges
        │                                   ▲
        │  PHASE A: release (push-down)     │
        │  squash snapshot + tag            │
        ▼                                   │
        └────────── PHASE B: sync-back ─────┘
                   (pull-up, selective)
```

- **Two products, not two branches of one product.** Workflow triggers,
  `.gitmodules` branch fields, and the plugin gitlink are branch-owned and
  deliberately differ.
- **Content descends by snapshot transplant (Phase A); ascends selectively
  (Phase B).** No cherry-picks Development→Current. Hotfixes Current→Development
  DO use `cherry-pick -x`.
- **The plugin submodule syncs FIRST in every phase** - the parent's gitlink
  must reference the plugin's tip, so bottom-up always.

---

## PART 1 - PHASE A: RELEASE (Development → Current), action-by-action

### A0. Preconditions (verify or ABORT)

```
[ ] P working tree CLEAN        (git status --porcelain empty - V8)
[ ] S working tree CLEAN
[ ] Versions bumped everywhere on Development (binary 5 spots + plugin)
[ ] Release notes finalized at Maintain/release-notes-vX.Y.Z.md
[ ] Gates green on Development: cargo build/test/clippy, ruff, pyright, deny
[ ] Tags checked free: git tag | grep 'Aphrodite/vX.Y.Z' → empty
[ ] Branch-identity audit clean on BOTH refs (I11 - see B4)
```

### A1. Plugin sync (S-Development → S-Current) - ALWAYS FIRST

Action 1: confirm plugin clean

```
git -C plugins/aphrodite status --porcelain      # empty or ABORT
```

Action 2: switch plugin to Current + ff-pull

```
git -C plugins/aphrodite checkout Current
git -C plugins/aphrodite pull --ff-only Source Current
```

Action 3: STAGE the whole Development tree as one snapshot (NO commit yet)

```
git -C plugins/aphrodite merge --squash Development
```

Action 4: review the staged set in VSCode - selective pick per file

```
# keep: version bump, docs, directives, real changes
# discard: anything dev-only (unstage = git restore --staged <file>)
git -C plugins/aphrodite status --short           # what's staged
```

Action 5: commit the snapshot + push

```
git -C plugins/aphrodite commit -m "release: sync vX.Y.Z"
git -C plugins/aphrodite push Source Current
```

Action 6 (at the END of the whole release, NOT here): tag on Current

```
git -C plugins/aphrodite tag vX.Y.Z              # e.g. v2.1.4
git -C plugins/aphrodite push Source vX.Y.Z
```

### A2. Parent sync (P-Development → P-Current) - SECOND

Action 7: confirm parent clean

```
git status --porcelain                            # empty or ABORT
```

Action 8: switch parent to Current + ff-pull

```
git checkout Current
git pull --ff-only Source Current
```

Action 9: STAGE Development as one snapshot (NO commit yet)

```
git merge --squash Development                    # or: git merge --squash <cutoff-sha>
```

Action 10: RESTORE the branch-owned identity files (Current keeps its own)

```
git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite .hermes bench
# .gitmodules → stays "branch = Current" on Current
# workflows → stay [Current]-triggered (Check.yml keeps Current's trigger + NO Test job)
# .hermes (incl. skills/), bench → stay OUT of Current (dev scaffolding never ships)
# plugins/aphrodite gitlink → restore, then float below
```

Action 11: float the plugin gitlink to the released plugin tip

```
git -C plugins/aphrodite fetch Source Current
git -C plugins/aphrodite checkout -B Current Source/Current
git add plugins/aphrodite
```

Action 12: VERIFY before committing (any failure = abort)

```
git diff HEAD -- .gitmodules .github/workflows .hermes bench   # EMPTY (I9)
git submodule status                                                    # no '+'
git grep -c 'chain_split' -- crates/aphrodite/src                      # per track decision
```

Action 13: commit + push + tag + GitHub release

```
git commit -m "release: sync vX.Y.Z from Development"
git push Source Current
git tag Aphrodite/vX.Y.Z && git push Source Aphrodite/vX.Y.Z
gh release create Aphrodite/vX.Y.Z --notes-file Maintain/release-notes-vX.Y.Z.md
   # NEVER inline backticks in --notes; always --notes-file
   # Build.yml auto-fires on refs/tags/Aphrodite/* → 12 artifacts (4 targets × bin+dylib+SUMS)
   # Publish.yml fires on tag push too: aphrodite + aphrodite-hermes publish steps run
   # on refs/tags/Aphrodite/* (only headroom-core needs workflow_dispatch + publish_crates)
```

Action 14: return the working copy to Development

```
git checkout Development
git -C plugins/aphrodite checkout Development
```

### A3. What ships vs what never ships (Current tree)

**Ships on Current:** crates/, plugins/ (as gitlink), docs, README, CHANGELOG, Maintain/scripts (non-bench), ruff.toml, rustfmt.toml, .prettier*, .vscode/settings.json
**Never ships (Development-only):** .hermes/ (incl. skills/), bench/, tests/, test_* files, auto-release.sh, bench examples (`[[example]]` blocks), dev notes

---

## PART 2 - PHASE B: SYNC-BACK (Current → Development), action-by-action

Purpose: bring release-line fixes/config back into Development so 1.5.0+ starts
from the same state. **Plugin FIRST, then parent** (same bottom-up rule).

### B0. Preconditions

```
[ ] Both repos on Development
[ ] Plugin tree clean (or phantom cleared - see B1 Action 2)
[ ] Branch-identity audit clean on BOTH refs (I11 - see B4)
```

### B1. Plugin sync-back (S-Current → S-Development)

Action 1: switch plugin to Development + ff-pull

```
git -C plugins/aphrodite checkout Development
git -C plugins/aphrodite pull --ff-only Source Development
```

Action 2: clear any phantom self-referential gitlink (auto-committer signature)

```
git -C plugins/aphrodite ls-files -s -- plugins/aphrodite    # 160000 → own HEAD = phantom
git -C plugins/aphrodite rm --cached plugins/aphrodite
git -C plugins/aphrodite status --short                     # clean
```

Action 3: STAGE Current's tree as one snapshot (NO commit)

```
git -C plugins/aphrodite merge --squash Source/Current
```

Action 4: VSCode selective pick - the user's decision point

```
# keep: formatter-aligned __init__.py, real fixes
# decide: test deletions - Development KEEPS tests (they're Development's job),
#         so typically UNSTAGE/DISCARD the 'D tests/...' entries
# discard: anything unwanted (git restore --staged <file>)
```

Action 5: commit + push (only after the user's picks)

```
git -C plugins/aphrodite commit -m "chore: sync Current release-line work back to Development"
git -C plugins/aphrodite push Source Development
```

### B2. Parent sync-back (P-Current → P-Development)

Action 6: STAGE Current's tree (NO commit)

```
git merge --squash Source/Current
```

Action 7: RESTORE Development's own identity (Development keeps its triggers)

```
git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite vendor/headroom
# .gitmodules → "branch = Development" on Development
# workflows → [Development]-triggered (Check.yml keeps the TEST job here)
# gitlinks → Development's own (or re-float after plugin B1)
```

Action 8: VSCode selective pick

```
# keep: ruff.toml, rustfmt.toml ignore, .vscode/settings.json, README badge,
#       finalized release notes, directives markdown
# reject: test-free deletions (Development keeps tests), Current-only CI edits
```

Action 9: verify + commit + push

```
git diff HEAD -- .gitmodules .github/workflows       # EMPTY (Development identity intact)
git submodule status                                  # no '+'
git commit -m "chore: sync Current release-line work back to Development"
git push Source Development
```

### B3. The selection benchmark (what transfers)

```
TRANSFERS (shared content):        STAYS (branch-owned identity):
  code fixes                        .github/workflows triggers ([Dev] vs [Current])
  formatter config (ruff/rustfmt)   .gitmodules branch fields
  .vscode/settings.json             plugin gitlink (each branch floats its own)
  README badge bumps                test-free deletions (Development KEEPS tests)
  release notes, CHANGELOG          Current-only CI edits (Test job removal)
  directives markdown
```

### B4. Branch-identity audit - MANDATORY pre-sync/pre-tag gate (I11)

Run BEFORE any merge, sync, or tag in EITHER phase (A0 and B0 precondition).
Read-only ref access only - NEVER checkout the other branch. Fetch both refs,
scan BOTH sides for [Development]/[Current] identity leaks, and refuse to
proceed on any hit. The Auto.yml class of leak: a workflow on one branch whose
push target names the OTHER branch.

```
git fetch Source
# 1. Workflow triggers + push targets (Auto.yml's `branch:` is branch-owned identity)
git grep -n -E 'branch: (Current|Development)|branches: \[(Current|Development)\]' Source/Development -- .github/workflows
git grep -n -E 'branch: (Current|Development)|branches: \[(Current|Development)\]' Source/Current -- .github/workflows
#    triggers must match the branch they live on ([Development] vs [Current]);
#    Auto.yml's push target is Current on BOTH copies (G5-05: the heartbeat
#    touches only the CI-ignored .github/Update.md path - a sanctioned
#    Current-side exception to I10). A push target of Development on either
#    copy = LEAK (ABORT).
# 2. .gitmodules branch fields must match the branch they live on
git show Source/Development:.gitmodules | grep -E '^branch'
git show Source/Current:.gitmodules | grep -E '^branch'
#    plugins/aphrodite -> Development on Development, Current on Current;
#    vendor/headroom + vendor/rtk -> Current on BOTH lines (by design, V7).
# 3. Gitlink targets must resolve to the branch they belong on
git -C plugins/aphrodite branch --contains "$(git ls-tree Source/Development plugins/aphrodite | awk '{print $3}')"
git -C plugins/aphrodite branch --contains "$(git ls-tree Source/Current plugins/aphrodite | awk '{print $3}')"
# 4. Keyword scan for branch-identity statements in the wrong branch's files
git grep -n -E 'Current|Development' Source/Development Source/Current -- .github/workflows .gitmodules
#    (docs/README tree/Current links, release notes, and taxonomy notes are
#    content describing the dual-line model, not identity - do not flag them)
```

Any hit where a file's branch-owned identity (workflow trigger/push target,
.gitmodules branch field, gitlink target, branch-identity statement) does not
match the branch it lives on = **ABORT the ceremony** - record it in
CEREMONY-AUDIT.md, fix the offending branch separately, re-run the audit clean,
then proceed.

---

## PART 3 - TAGGING RULES (the immutable step)

- **Tags exist ONLY on Current.** Development never carries release tags (V5).
- **Sequence: plugin tag first, then parent tag.** `v2.1.4` on S-Current, then
  `Aphrodite/v1.4.3` on P-Current.
- **Tag AFTER the final tip is settled, at the END of the ceremony** - not
  mid-flight. Every code/config fix after a tag forces a re-tag (delete +
  recreate + force-push), which re-triggers Build/Publish. Minimize by finishing
  ALL changes before tagging.
- **Tag types:** follow the repo's convention (`git cat-file -t <existing-tag>`
    - annotated = use `-a -m`).
- **Re-tagging is allowed but deliberate:** `git tag -d X` + `git push Source
:refs/tags/X` + recreate + `git push -f`. Only when the tag predates a
  required fix.
- **Prereleases** use tags (`Aphrodite/vX.Y.Z-rc.1`), never branches.

---

## PART 4 - RELEASE / CI TRIGGERS (what fires where)

**Trigger:** push to Development
**Workflow:** Check.yml `[Development]`
**What it does:** full CI: fmt (nightly), check, clippy, deny, ruff, pyright, **tests**

---

**Trigger:** push to Current
**Workflow:** Check.yml `[Current]`
**What it does:** CI minus Test job (test-free line): fmt, check, clippy, deny, ruff, pyright

---

**Trigger:** tag `Aphrodite/v*`
**Workflow:** Build.yml
**What it does:** **12 artifacts**: 4 targets × (binary + libaphrodite_hermes + SHA256SUMS) + source zips

---

**Trigger:** tag `Aphrodite/v*`
**Workflow:** Publish.yml
**What it does:** fires and publishes to crates.io when `workflow_dispatch` + `publish_crates: true` (manual, deliberate) OR on a plain tag push - the `aphrodite`/`aphrodite-hermes` publish steps carry ` |     | startsWith(github.ref, 'refs/tags/Aphrodite/')`and DO fire on tag; only`aphrodite-headroom-core` is truly dispatch-only (order: aphrodite-headroom-core → aphrodite → aphrodite-hermes)

---

**Trigger:** plugin tag `vX.Y.Z`
**Workflow:** (Aphrodite-Hermes repo)
**What it does:** plugin release marker; plugin has no CI

`download.sh` resolves the binary by `BINARY_VERSION` (arg → file → Cargo.toml →
GitHub latest) and builds URL `releases/download/Aphrodite%2Fv{V}` - works for
any tagged release with identical asset names.

---

## PART 5 - THE FORMATTER CONTRACT (why CI and VSCode must agree)

- Repo `rustfmt.toml` uses **nightly-only unstable options**
  (`space_after_colon = false` etc.). CI pins `nightly-2026-05-01`.
- Stable rustfmt / rust-analyzer internal IGNORE those → space-after-colon →
  drift. The nightly-rustfmt contract is enforced in CI workflows (the
  `[rust]` block in `.vscode/settings.json` only selects rust-analyzer as the
  formatter - it does NOT override the toolchain, so VSCode == CI only when
  rust-analyzer is configured to the pinned nightly; the repo's
  `rust-toolchain.toml` (stable 1.96.0) silently ignores the unstable
  options, so the drift the contract exists to prevent is currently only
  caught by CI).
- `ruff.toml`: explicit (line-length 100, double quotes, space indent, LF).
  NOTE: it has NO `extend-exclude` for `crates/aphrodite/templates/**` - the
  embedded template is kept ruff-formatted, which is exactly why the
  byte-identity guard holds. Do not add an exclude; format the template.
- **Shim template rule:** `crates/aphrodite/templates/__init__.py` MUST stay
  byte-identical to `plugins/aphrodite/__init__.py` (setup.rs asserts).
  Format the PLUGIN first, then `cp` to the template. Never format the template
  directly. (`rustfmt.toml` ignores `vendor/`, `target/`, and codegen dirs -
  templates are irrelevant to rustfmt since it only formats `.rs`.)

---

## PART 6 - AUTO-COMMITTER HANDLING (operational reality)

The user runs an auto-committer that sweeps working-tree changes into commits.
It can stage a **phantom self-referential gitlink** inside submodules (a 160000
entry named after the submodule, pointing at its own HEAD, with no dir on disk).
Symptom: `git status` shows `A plugins/aphrodite` / `AD vendor/headroom`.

- Never fight it. Clear the phantom: `git rm --cached <path>` inside the
  submodule, verify `git ls-files -s` has no 160000 entry.
- It may commit mid-review staged sets - verify with `git log`, not just
  `git status`.
- It commits on the checked-out branch - keep Current-only work inside the
  ceremony window, working copy returns to Development after (I10).

---

## PART 7 - INVARIANT CHECKLIST (run after every phase)

```
I1  both repos on a named branch
I2  git submodule status: no '+'; git status: no 'M <submodule>'
I3  Current tree == tagged released content
I4  Development append-only (no rebase, no reset)
I5  .gitmodules branch field matches the branch it lives on
I6  workflow triggers match the branch ([Development] / [Current])
I7  upward picks carry -x + .hermes/picks manifest entry
I8  versions monotonic, never reused
I9  protected paths clean after any transplant (diff HEAD empty on them)
I10 working copy on Development; Current touched only inside the ritual
I11 branch-identity audit: no [Development]/[Current] identity leak on EITHER
    ref before any merge/sync/tag (workflow triggers + push targets,
    .gitmodules branch fields, gitlink targets - see B4)
D1  selective boundary honored (chain-split absent/present per track)
D2  chain-split opt-in OFF in shipped config (aphrodite.toml)
```

---

## PART 8 - THE COMPLETE RELEASE CYCLE (one pass, end to end)

```
PHASE A (release):
  A0 checks (incl. B4 branch-identity audit) → A1 plugin sync+push → A2 parent sync+push →
  A2 tags (plugin vX.Y.Z, parent Aphrodite/vX.Y.Z) → gh release → A14 return
PHASE B (sync-back, after release is published):
  B1 plugin squash-staging → VSCode pick → commit+push →
  B2 parent squash-staging → restore identity → VSCode pick → commit+push →
  B4 audit gate re-run (I11)
NEXT CYCLE:
  Development continues (1.5.0 work), bump versions, repeat Phase A.
```

**The one rule that ties it together:** the plugin moves first in both
directions; identity files never cross; tests/bench/dev-scaffolding only exist
on Development; tags only on Current; every transplant is a reviewable staged
snapshot, never an automatic merge.
