# Pitfalls - scar catalog with probes

Real ceremony failures kept as test cases. Each entry names the failure, the
mechanism, the probe that detects it, and the repair. SKILL.md holds the
standing rule; this file holds the mechanics.

## Phantom self-referential gitlink (recurs)

A stray mode-160000 entry inside the submodule pointing at its OWN commit,
swept in by the auto-committer alongside unrelated work. The submodule has
NO `.gitmodules`, so any 160000 entry is self-referential.

Symptoms:

- `git submodule status` INSIDE the submodule fails with `fatal: no
submodule mapping found in .gitmodules for path '<submodule-name>'`.
- The remote shows a nested `plugins/aphrodite` folder that should not
  exist.
- `git clone --recurse-submodules` dies with `fatal: No url found for
submodule path 'X/X' in .gitmodules`.

Detect inside the submodule:

```sh
git -C -s < submodule > ls-files | grep 160000 # a hit = phantom
```

Repair:

```sh
git -C <submodule> rm --cached <path>
# verify the grep is empty + git status clean; commit, push
git -C <submodule> ls-files -s | grep 160000 || true
```

Float parent gitlinks OFF any commit that contains it (`git ls-tree <ref>
<path>` for 160000). The auto-committer RE-STAGES a fresh phantom pointing
at the new HEAD after you clear it - re-run the `ls-files -s | grep 160000`
check after EVERY subsequent submodule commit. A clean check at the end of
the session is the real pass criterion, not one removal.

## The git hooks are GONE - never re-create them

The entire `.githooks/` set (pre-commit, post-commit, post-checkout,
post-merge, lib/*) was REMOVED because the auto-bump hooks were the
resurrection vector for the phantom self-referential gitlink: `post-commit`/
`post-checkout` re-staged a 160000 entry named after the submodule inside the
submodule after every manual clear, and `package.json`'s `prepare` re-installed
the hooks on every npm install. Removal = `git rm -r .githooks`, unset
`core.hooksPath` in parent AND submodule, strip the `prepare` script, delete
the `.gitattributes` `.githooks/*` lines, clear the phantom with
`git -C <submodule> rm --cached <path>`.

With no hooks, branch anchoring, gitlink auto-bump, and the commit gate are
gone: submodule pins are verified by hand (`git submodule status` shows no
'+' = I2), and a detached submodule HEAD is fixed manually (`git -C
plugins/aphrodite checkout Development`). Re-creating any hook or the
prepare script is a ceremony violation.

## Auto-committer races

The auto-committer sweeps working-tree changes (including staged squash sets
and gitlink bumps) into commits and pushes. Never fight it; verify final
state with `git log`/`git submodule status`, not `git status`. A squash
staged for VSCode review can be swept mid-review - the content survives as a
commit.

## `git cherry-pick --continue` commits leftover conflict markers

A file with TWO conflict regions: resolving the first and continuing commits
the still-marker'd second region into the branch. Grep the committed file
set for `<<<<<<<` after every `--continue`:

```sh
git diff HEAD~1 HEAD --name-only | xargs grep -l '<<<<<<<' || true
```

Fix + `git commit --amend --no-edit` when one slipped through.

## Empty cherry-pick is "already contained", not an error

`nothing to commit, working tree clean` means the change is already in HEAD
via an earlier pick or merge resolution - verify with
`git diff <HEAD> <source> -- <paths>` and skip the commit instead of
aborting in confusion.

## Submodule-first ordering is mandatory

In every phase (release AND sync-back): plugin first, then parent, so the
parent's gitlink references the released plugin in one pass.

## Removing a subsystem = sweep the whole tree, then record the absence

Skills, profiles, s2/navigation, and the installers were each removed across
sessions: the self-heal schema (`layout_schema.json`), embedded templates,
config example, README tree diagrams, bench scripts, release skills, and the
classification passes ALL carry references to the removed thing. Delete the
code AND every reference (schema entries, feature gates + their cfg
branches, docs, tests that probe it), then state the ABSENCE as a ceremony
rule (e.g. 'profiles never ship', 'skills live dev-side', 'directives ship
in the binary') so a future session does not re-add it or treat the empty
cherry-pick as an error.

The sweep includes the FFI symbol list: removing a `#[no_mangle]` export
without updating the dylib's expected-symbol list leaves `aphrodite_rebuild`
(and any dlsym-based check) dying with "missing an expected symbol" against
the fresh build - grep for the symbol name in the tooling/check code, not
just the crate's lib.rs, and remember the live session may still hold the
OLD dylib until Hermes is restarted.

## The setup flow needs a key, not the plugin

Proxy spawn dies with "no API key configured" when `APHRODITE_API_KEY` /
toml `api_key` is absent - Hermes' provider config is NOT reused. See
`references/plugin-lifecycle.md` for key sourcing and install layouts.

## Package READMEs render on crates.io

The crate-dir READMEs (`crates/<crate>/README.md` - what cargo
auto-includes), NOT the root README, are what crates.io shows. Relative
links there render as `blob/HEAD`. Write every link absolute to the file's
OWN branch context - infer it per file, never blanket-assume: the branch
that owns the file AFTER the merge (Development files → `tree/Development`,
files staged for the Current distribution line → `tree/Current`).

## Tag immutability

Create the tag only after the exact release-sync commit and all tag
prerequisites (B4, Gate R7, asset contract). Re-tagging re-fires
Build/Publish and rewrites history's evidence.

## Embedded templates drift

Stale templates mean fresh `aphrodite setup` writes a config missing keys
the engine reads - refresh before release (Step P3); the shim
`templates/__init__.py` must stay byte-identical to the live plugin
`__init__.py` (setup.rs asserts it).
