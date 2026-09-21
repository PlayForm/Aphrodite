# Dual-Track Release Strategy - v1.4.3 + v1.5.0

**Status: DESIGN / PLANNING ONLY.** No commits, pushes, or tags were made.
This document decides the selective-promotion boundary, tag/promote mechanics,
and the git-graph shape for releasing **v1.4.3** and **v1.5.0** separately
through `Current`, per the two-line model confirmed by the Perplexity feedback
(snapshot transplants down, `-x` cherry-picks up, `promote/vX.Y.Z` = optional
review scaffolding only). It extends `aphrodite-branch-release-flow` +
`branch-flow-protocol`; every command below follows that ceremony unless a
deviation is explicitly flagged in §10.

## 1. Verified repository state (2026-09-16, read-only)

**Property:** HEAD

**Parent P (`PlayForm/Aphrodite`):** `4f14235` on Development (Source/Development in sync)

**Submodule S (`plugins/aphrodite`, `PlayForm/Aphrodite-Hermes`):** `b6fecad` on Development (Source/Development in sync)

---

**Property:** Current tip

**Parent P (`PlayForm/Aphrodite`):** `0028705` (= Source/Current, Source/HEAD)

**Submodule S (`plugins/aphrodite`, `PlayForm/Aphrodite-Hermes`):** `5b62d68` (= Source/Current, Source/HEAD)

---

**Property:** Merge-base Dev/Current

**Parent P (`PlayForm/Aphrodite`):** `3100948`

**Submodule S (`plugins/aphrodite`, `PlayForm/Aphrodite-Hermes`):** `0c1f130`

---

**Property:** Tags

**Parent P (`PlayForm/Aphrodite`):** `Aphrodite/v0.x … Aphrodite/v1.4.2` - **no `Aphrodite/v1.4.3`, no `Aphrodite/v1.5.0`**

**Submodule S (`plugins/aphrodite`, `PlayForm/Aphrodite-Hermes`):** `v1.62.x, v2.0.x, v2.1.0 … v2.1.2` - **no `v2.1.3`**

---

**Property:** Gitlink

**Parent P (`PlayForm/Aphrodite`):** P→S = `b6fecad` (clean, no `+` in `git submodule status`)

**Submodule S (`plugins/aphrodite`, `PlayForm/Aphrodite-Hermes`):** S has no submodules

---

**Property:** Version state

**Parent P (`PlayForm/Aphrodite`):** binary 1.4.3 (bump `5d8c09a`), plugin 2.1.3 (`plugin.yaml`, bumped in S `350e8c2`)

**Submodule S (`plugins/aphrodite`, `PlayForm/Aphrodite-Hermes`):** `plugin.yaml: 2.1.3`, min_hermes 0.16.0

---

**Property:** Protected-state

**Parent P (`PlayForm/Aphrodite`):** Current lacks `.hermes/`, `bench/`, `skills/` (Current-only cleanup `cda5d0a`, `c876fbe`); Current-only `.githooks/pre-commit` deletion + `.gitignore`/root `Cargo.toml` trims (`0028705`)

**Submodule S (`plugins/aphrodite`, `PlayForm/Aphrodite-Hermes`):** Current has 1 extra commit `5b62d68` (hook-battery test removal)

Tag collision check: `git tag | grep -E 'Aphrodite/v1\.4\.3|Aphrodite/v1\.5\.0'` and
`git -C plugins/aphrodite tag | grep 'v2\.1\.3'` are both empty today → all three
planned tags are free.

**Conflict prediction (1.4.3 squash, base `3100948` → cutoff):** Development made
no post-base changes to `.githooks/pre-commit`, root `Cargo.toml`,
`Maintain/Format.sh`, `.vscode/*`, `.github/Update.md` → Current's deletions of
those survive the transplant with no conflict. Dev changed `.gitignore`
(+5/−1, commits `48abe46`/`4b06e79`) vs Current's 4-line trim in `0028705` →
the only predicted line-level overlap; resolve per V2 (§9) keeping Current's
content. Dev changed `.github/workflows/Check.yml`, `.gitmodules`, and 68
`.hermes/`/`bench/`/`skills/` files → all reverted by the protected-path restore
(§7, step 5).

## 2. Decision (a) - the selective-promotion boundary

**1.4.3 ships ONLY the pure 1.4.2→1.4.3 prep (release pipeline, directives,
hooks, plugin hardening, benchmark tooling, docs) - WITHOUT chain-split,
WITHOUT Tier-1 teaching loop, WITHOUT Tier-3 error hints. All of those ship
in 1.5.0.**

The 1.4.3 prep freeze is the handoff state (`RELEASE-HANDOFF-1.4.3.md`): the
prep commits up to and including **`73b3272`** (docs refactor) - the last
commit BEFORE chain-split landed. Commits after that freeze split as follows -
the boundary commit is **`73b3272`** (chain-split core `475917d` and
everything after it belongs to 1.5.0):

**Track:** **1.4.3** (through cutoff `73b3272`)
**P commits (oldest → newest):**

```text
9288d9e ad010af 6d6287e 405817c 5056669 4408ea4 c1ad84a 208f8c8 46f1107 ccd8074 5d8c09a 8afe8e0 db12c86 48abe46 5effa73 6f531a5 fb63aa5 4b06e79 aa10794 73b3272
```

**Content:** frozen prep (bump `5d8c09a`, notes `8afe8e0`, draft `db12c86`, gitignore `48abe46`, bench suite `5effa73`, handoff `6f531a5`, CCR notes `fb63aa5`, AGENTS.md `4b06e79`, scope `aa10794`, docs `73b3272`) - **NO chain-split** (verified: `git grep chain_split 73b3272 -- crates/` empty; no `chain_split.rs` at that tree)

---

**Track:** **1.5.0** (after cutoff, current tip)

**P commits (oldest → newest):** `475917d d26dd89 8521b27 85771ee 50ff098 5e18dd3 da57c79 9f70cc0 f94c4fa` + gitlink bumps

**Content:** **chain-split core `475917d`** (`chain_split.rs` + wiring, `chain_split = false` default), output-invisibility `d26dd89`, directive prose `8521b27`, continuation notes `5e18dd3`, **Tier-1 teaching loop `85771ee`**, **Tier-3 error hints `50ff098`**, retrieve round-trip fix `da57c79`, .gitignore commits

S-side has **no dual track**: all 6 post-v2.1.2 S commits (`1086c4a 350e8c2
4238b38 15e03d8 90e7c9f b6fecad`) ship in plugin **v2.1.3** with 1.4.3. The
tier work is P-side (`feat(aphrodite)`), so 1.5.0 reuses the same S-Current tip
unless S changes (then plugin v2.1.4).

Tree-delta proof of the boundary: `git diff --stat 8521b27 4f14235` = **2 files,
+436/−2** - exactly the two tier commits. `git grep adapt_chain_split_threshold`
and `git grep error_hint` on Current (`0028705`) both return nothing today.

Carve-outs (in the 1.4.3 commit list but NOT in the Current tree): `bench/`
(`5effa73` - DEV-INFRA per scope), `.hermes/` docs, `skills/` (`46f1107`/`ccd8074`),
`.github/workflows` (`208f8c8`), `.gitmodules`, gitlink - all protected (§7 step 5).

## 3. Decision (b) - promotion mechanism per release

**Both releases promote DIRECTLY on `Current` - no `promote/vX.Y.Z` branch.**

Rationale (skill table + feedback): both `Current` branches are unprotected;
review is not mandatory in this flow (single-writer ritual, all gates green on
Development before the sync); the skill's "maximum simplicity" row is direct
promotion. The PR branch is review scaffolding, not architecture - reserve it
for a future release that genuinely requires review. The full PR variant is
documented in §8 so it is executable if that need appears; the only ceremony
difference is where the snapshot commit is created and that the branch is
deleted after tagging.

## 4. Decision (c) - tag naming (collision-verified)

| Track    | Parent tag                   | Plugin tag                                   | Notes                                                                   |
| -------- | ---------------------------- | -------------------------------------------- | ----------------------------------------------------------------------- |
| 1.4.3    | `Aphrodite/v1.4.3`           | `v2.1.3`                                     | prefix `Aphrodite/v` matches v1.4.0-v1.4.2; S prefix `v` matches v2.1.x |
| 1.5.0    | `Aphrodite/v1.5.0`           | - (reuses `v2.1.3` tip; **no second S tag**) | bump plugin to `v2.1.4` only if S-side changes land before 1.5.0        |
| Fallback | `Aphrodite/v1.4.3-rc.1` etc. | -                                            | prerelease via tag, never a branch (protocol axiom 5)                   |

All tags verified free (§1). Tags exist **only on Current** (I5/V5): Development
never carries release tags; the 1.4.3/1.5.0 version claims happen via the
Development bump commits (`5d8c09a` exists; a new `chore(release): bump v1.5.0`
commit is required before the 1.5.0 sync - I8, never reuse a claimed number).

## 5. Decision (d) - how the two tracks appear in git graphs

- **Development** keeps the ENTIRE two-track history, linear and append-only
  (I4): prep freeze → chain-split core → tiers → bookkeeping → future bumps.
  No tags, no promote branches, no snapshot commits here.
- **Current** shows TWO SEQUENTIAL snapshot commits (one per release), each
  tagged, in release order. Each snapshot = one squashed tree transplant
  (`release: sync vX.Y.Z from Development`); promote branches (if ever used)
  are deleted after tagging, so they never linger in the graph.
- **The submodule graph** shows ONE new sync commit (`release: sync v2.1.3`),
  tagged, on S-Current; both parent snapshots float the gitlink to that same
  commit.
- The 1.4.3 snapshot's parent on Current is `0028705` (previous released
  state); the 1.5.0 snapshot's parent is exactly the 1.4.3 snapshot commit -
  machine-checkable as `git rev-parse Aphrodite/v1.5.0^ == Aphrodite/v1.4.3`.

```
PARENT  PlayForm/Aphrodite
 Development (workshop; append-only; NO tags)      Current (distributed; tags ONLY here)
 * 4f14235  bump gitlink → b6fecad                  *
 * 2f5dfea  bump gitlink → 90e7c9f                  * A release: sync v1.5.0 from Development  ★ Aphrodite/v1.5.0
 * d798c4f  (empty msg, no tree)    1.5.0 track ──► * B release: sync v1.4.3 from Development  ★ Aphrodite/v1.4.3
 * 50ff098  Tier 3 error hints                      * 0028705 (previous Current tip; 1.4.2-era released state)
 * 85771ee  Tier 1 teaching loop                    * c876fbe/cda5d0a/fd6c939 (Current-owned cleanup)
 * 8521b27  chain-split docs   ◄─ 1.4.3 CUTOFF ───── * 3100948 …shared history…
 * d26dd89  chain-split output invisible + dirs      *
 * 5e18dd3  continuation notes (1.4.3 prep)
 * 475917d  chain-split core (opt-in, default OFF)
 * 73b3272  docs · aa10794 scope doc · 4b06e79
 * fb63aa5  CCR notes · 6f531a5 handoff doc
 * 5effa73  bench suite (DEV-INFRA → not on Current)
 * 48abe46  gitignore · db12c86 draft · 8afe8e0 notes
 * 5d8c09a  bump v1.4.3   (1.4.3 prep freeze area)
 * …        pre-1.4.3 history (shared up to 3100948)

SUBMODULE  plugins/aphrodite (PlayForm/Aphrodite-Hermes)
 S-Development                              S-Current
 * b6fecad  drop phantom gitlink             * C release: sync v2.1.3   ★ v2.1.3  ◄── gitlink of BOTH A and B
 * 90e7c9f  remove phantom gitlink  ───────► * 5b62d68 (prev S-Current tip; Current-owned)
 * 15e03d8  directives restyle                * 0c1f130 …shared history…
 * 4238b38  marker handling decision-based
 * 350e8c2  perf probe + plugin.yaml 2.1.3
 * 1086c4a  README fix
 * 3c6f600  ★ v2.1.2 (shared base)
```

Note: the 1.4.3 snapshot commit B is created from `8521b27`'s tree - the tier
commits `85771ee`/`50ff098` exist only on Development (and later on Current
via A). No commit SHAs are duplicated across branches (no cherry-picks down,
no merge commits; V2/V3 traps avoided by design).

## 6. Two-track timeline

1. **T-0 (now):** 1.4.3 fully prepared on Development (gates green, handoff/scope
   docs committed). Current untouched. Nothing tagged.
2. **Track A - v1.4.3 + plugin v2.1.3:** S sync first (§7.1), then P sync at the
   cutoff `8521b27` (§7.2). Tags `v2.1.3` + `Aphrodite/v1.4.3` on Current only.
   Working copy returns to Development (I10).
3. **Between tracks:** Development keeps accumulating. If a hotfix is born on
   Current (1.4.3 regression), cherry-pick up with `-x` + `.hermes/picks/`
   manifest entry (I7). No Current work otherwise.
4. **Track B - v1.5.0:** prep on Development (bump 1.4.3→1.5.0, release notes
   `Maintain/release-notes-v1.5.0.md`, gates/build/test, commit
   `chore(release): bump v1.5.0`). Then P sync at the Development tip (§7.3).
   Tag `Aphrodite/v1.5.0` on Current. No S sync / no new S tag (plugin reuses
   `v2.1.3`) unless S-side changes landed (then v2.1.4 sync first, same §7.1
   sequence).
5. **Order is mandatory:** 1.4.3 must be tagged before 1.5.0 (I8 monotonic,
   sequential snapshots on Current - single-writer axiom 7).

## 7. Command sequences (per `aphrodite-branch-release-flow`, adapted)

### 7.1 Release 1 - plugin sync (S-Dev → S-Current), tag `v2.1.3`

Run from P root; S commands via `-C`. S has no dual track and no protected
paths, so the S squash uses the S-Development **tip** (`b6fecad`):

```sh
git -C plugins/aphrodite status --porcelain # clean or ABORT (V8)
git -C plugins/aphrodite checkout Current
git -C plugins/aphrodite pull --ff-only Source Current
git -C plugins/aphrodite merge --squash Development # tree@b6fecad; 5b62d68 survives
git -C plugins/aphrodite status --porcelain         # review staged diff (6 commits' content)
git -C plugins/aphrodite commit -m "release: sync v2.1.3"
git -C plugins/aphrodite push Source Current
git -C plugins/aphrodite tag v2.1.3 # collision-free (verified §1)
git -C plugins/aphrodite push Source v2.1.3
```

### 7.2 Release 1 - parent sync at the CUTOFF `8521b27`, tag `Aphrodite/v1.4.3`

```sh
git status --porcelain                                                                    # clean or ABORT (V8)
git checkout Current && git pull --ff-only Source Current                                 # tip = 0028705
git merge --squash 8521b27                                                                # DEVIATION 1: cutoff SHA, not Development tip
git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite .hermes bench skills # DEVIATION 2
git -C plugins/aphrodite fetch Source Current
git -C plugins/aphrodite checkout -B Current Source/Current # float to the v2.1.3 sync commit
git add plugins/aphrodite
# ---- verify before commit ----
git diff HEAD -- .gitmodules .github/workflows .hermes bench skills        # empty (I9)
git submodule status                                                       # no '+' (I2)
git grep -c 'adapt_chain_split_threshold' -- crates/aphrodite/src/state.rs # 0 (no Tier 1, D1)
git grep -c 'error_hint' -- crates/aphrodite/src/chain_split.rs            # 0 (no Tier 3, D1)
git show :aphrodite.toml | grep 'chain_split = false'                      # opt-in OFF (D2)
git diff --stat 0028705 HEAD                                               # expected: ~80-100 files (see §1 prediction)
# ---- commit / push / tag / release ----
git commit -m "release: sync v1.4.3 from Development"
git push Source Current
git tag Aphrodite/v1.4.3 && git push Source Aphrodite/v1.4.3
# GitHub release from the tag, notes via --notes-file Maintain/release-notes-v1.4.3.md
#   (add a "chain-split (opt-in, default OFF)" bullet; Build.yml fires on refs/tags/Aphrodite/*)
git checkout Development # I10 - working copy home
```

Conflict rule if any (V2): keep Current's content (e.g. `0028705`'s `.gitignore`
trim), do **not** pick the deletion up to Development (it is Current-owned
public-tree cleanup, not a hotfix); verify I3 after.

### 7.3 Release 2 - prep v1.5.0 on Development, then sync at the tip

Prep (Development, after 1.4.3 is published - order mandatory):

```sh
# P bump 1.4.3 → 1.5.0: crates/aphrodite/Cargo.toml, crates/aphrodite-hermes/Cargo.toml
#   (package + dep), package.json, plugins/aphrodite/BINARY_VERSION, README badges (I8)
# notes: Maintain/release-notes-v1.5.0.md + .hermes/release-notes/v1.5.0-draft.md
# gates: cargo clippy -p aphrodite -- -D warnings; ruff check plugins/aphrodite/
#        cargo deny check advisories; python3 -c "import aphrodite"
# build/test: cargo build --release -p aphrodite -p aphrodite-hermes; cargo test -p aphrodite -p aphrodite-hermes
git add -A && git commit -m "chore(release): bump v1.5.0"
# S-side: NO change expected → skip §7.1; gitlink floats to the existing v2.1.3 tip
#   IF S changed: bump plugin.yaml 2.1.3→2.1.4 and run §7.1 with v2.1.4 BEFORE the P sync
```

P sync at the Development tip (includes `85771ee` + `50ff098` + bump):

```sh
git status --porcelain                                    # clean or ABORT (V8)
git checkout Current && git pull --ff-only Source Current # tip = the v1.4.3 sync commit
git merge --squash Development                            # tree@tip - standard ceremony, no cutoff needed
git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite .hermes bench skills
git -C plugins/aphrodite fetch Source Current
git -C plugins/aphrodite checkout -B Current Source/Current # = v2.1.3 commit (unchanged) unless §7.1 ran
git add plugins/aphrodite
# ---- verify before commit ----
git diff HEAD -- .gitmodules .github/workflows .hermes bench skills        # empty (I9)
git submodule status                                                       # no '+' (I2)
git grep -c 'adapt_chain_split_threshold' -- crates/aphrodite/src/state.rs # > 0 (Tier 1 present - D1 flipped)
git grep -c 'error_hint' -- crates/aphrodite/src/chain_split.rs            # > 0 (Tier 3 present)
git show :aphrodite.toml | grep 'chain_split = false'                      # still opt-in OFF (D2)
git commit -m "release: sync v1.5.0 from Development"
git push Source Current
git tag Aphrodite/v1.5.0 && git push Source Aphrodite/v1.5.0
# GitHub release --notes-file Maintain/release-notes-v1.5.0.md (Tier 1 + Tier 3 bullets)
git checkout Development # I10
```

## 8. PR-branch variant (NOT used for 1.4.3/1.5.0; documented for when review is mandatory)

Identical ceremony, snapshot commit created on `promote/vX.Y.Z` instead of
Current, branch deleted after tagging:

```sh
git checkout Current && git pull --ff-only Source Current
git checkout -b promote/v1.4.3 Current # or promote/v1.5.0
git merge --squash 8521b27             # cutoff for 1.4.3; `Development` for 1.5.0
git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite .hermes bench skills
# … float gitlink + verify as §7.2/§7.3 …
git commit -m "release: sync v1.4.3 from Development"
git push Source promote/v1.4.3 # open PR promote/v1.4.3 → Current; review; merge
git checkout Current && git pull --ff-only Source Current
git tag Aphrodite/v1.4.3 && git push Source Aphrodite/v1.4.3
git branch -D promote/v1.4.3 # delete - never lingers in the graph
git checkout Development
```

## 9. Invariant checklist (run after EACH release ritual - any failure blocks)

**#:** I1

**Predicate:** Both repos on a named branch; hooks float on detach

**Check (on the released state):** `git symbolic-ref -q HEAD` succeeds in P and S

---

**#:** I2

**Predicate:** No stale gitlinks

**Check (on the released state):** `git submodule status` shows no `+`; `git status` no `M plugins/aphrodite`

---

**#:** I3

**Predicate:** Current tree == tagged released content

**Check (on the released state):** `git diff 8521b27 Aphrodite/v1.4.3 -- . ':(exclude).hermes' ':(exclude)bench' ':(exclude)skills' ':(exclude).github' ':(exclude).gitmodules' ':(exclude)plugins/aphrodite'` empty (same for `Development→Aphrodite/v1.5.0`)

---

**#:** I4

**Predicate:** Development static: append-only, no rebase/reset

**Check (on the released state):** `git reflog` shows no reset/rebase since freeze

---

**#:** I5

**Predicate:** `.gitmodules` branch field matches branch

**Check (on the released state):** `grep branch .gitmodules` → `Current` on Current, `Development` on Development

---

**#:** I6

**Predicate:** Workflow triggers branch-specific

**Check (on the released state):** `grep -r branches .github/workflows` → `[Current]` on Current, `[Development]` on Development

---

**#:** I7

**Predicate:** Every upward pick carries `-x` + manifest entry

**Check (on the released state):** n/a unless a Current hotfix occurred; then `.hermes/picks/` has the entry

---

**#:** I8

**Predicate:** Versions monotonic

**Check (on the released state):** 1.4.3 → 1.5.0, plugin 2.1.3 once; `git log --format=%s                                                                                                                                                                      | grep bump` shows no reuse

---

**#:** I9

**Predicate:** Protected paths clean after transplant

**Check (on the released state):** `git diff HEAD -- .gitmodules .github/workflows .hermes bench skills` empty on Current

---

**#:** I10

**Predicate:** Working copy on Development; Current touched only in ritual

**Check (on the released state):** `git branch --show-current` == Development at session end (P and S)

---

**#:** D1

**Predicate:** Selective boundary honored

**Check (on the released state):** 1.4.3: `git grep adapt_chain_split_threshold Aphrodite/v1.4.3` and `git grep error_hint Aphrodite/v1.4.3` → nothing; 1.5.0: both present

---

**#:** D2

**Predicate:** chain-split opt-in OFF
**Check (on the released state):**

```sh
git show Aphrodite/v1.4.3:aphrodite.toml | grep 'chain_split = false'
```

---

**#:** D3

**Predicate:** Tracks sequential + parented

**Check (on the released state):** `git rev-parse Aphrodite/v1.5.0^` == `git rev-parse Aphrodite/v1.4.3`; `git rev-parse Aphrodite/v1.4.3^` == `git rev-parse 0028705`

## 10. Verification vs the skills - deliberate deviations (all intentional)

1. **DEVIATION 1 (the core dual-track mechanic):** the 1.4.3 parent sync uses
   `git merge --squash 8521b27` (the cutoff SHA) instead of the skill's
   `git merge --squash Development`. Required by selective promotion: the tier
   commits `85771ee`/`50ff098` must stay off Current until 1.5.0, and I4
   forbids rewinding Development to the cutoff. Squash of a non-tip commit is
   tree-based and valid; the 1.5.0 sync reverts to the standard tip squash.
2. **DEVIATION 2 (protected-path set extended):** the restore line adds
   `.hermes bench skills` to the skill's three paths, per the DEV-INFRA table
   in `RELEASE-SCOPE-1.4.3.md` (already removed from Current by `cda5d0a`/
   `c876fbe`; the squash would otherwise resurrect 68 files). `git checkout
HEAD -- <path>` deletes staged additions when HEAD lacks the path, which is
   exactly the needed behavior.
3. **NOTE 3 (no second plugin tag):** 1.5.0 reuses S-Current's `v2.1.3` tip -
   the skill's "sync plugin first" runs only when the gitlink target changes
   (I8: bump where the change ships). Plugin v2.1.4 is the trigger rule if S
   changes before 1.5.0.
4. **NOTE 4 (V2 conflict rule scoped):** on any squash conflict, keep Current's
   content and do NOT pick the deletion up to Development - the Current-owned
   deletions (`0028705`, `c876fbe`, `cda5d0a`) are public-tree cleanup
   divergence, not hotfix content; the superset rule applies to fixes, not to
   established identity divergence (protocol axiom 4).
5. **NOTE 5 (resolved prep inconsistency):** handoff said chain-split "default
   true"; verified `config_loader.rs:169` + `aphrodite.toml:55` say default
   FALSE → scope doc is authoritative; D2 enforces opt-in OFF in the release.
6. **NOTE 6 (known issue rides along):** the `__APHRODITE_SEG__` redirected-
   stdout marker pollution ships masked in 1.4.3 (feature OFF by default);
   the side-stream fix remains a 1.5.0 follow-up (scope OUT table).
7. **NOTE 7 (`d798c4f`):** empty-message, no-tree-effect commit in the 1.5.0
   track - harmless; optionally reworded via notes, never rewritten.

Everything else - submodule-first ordering, clean-tree abort (V8), protected
restore before gitlink float (V6), tag-on-Current-only (V5), no bump on
Current except hotfix-claims (V7), return-to-Development (I10) - matches the
skill ceremony exactly.

## 11. Post-release verification (after BOTH tracks)

```sh
git tag -l 'Aphrodite/v1.4.*' 'Aphrodite/v1.5.*'   # exactly the two new tags on Current
git -C plugins/aphrodite tag -l 'v2.1.*'           # v2.1.3 present, no duplicate
git log --oneline --first-parent Current | head -5 # A then B then 0028705
git submodule status                               # no '+' in P
git branch --show-current                          # Development everywhere
```
