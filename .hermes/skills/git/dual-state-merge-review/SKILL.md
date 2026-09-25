---
name: dual-state-merge-review
description: "Use when reviewing branch merges: staged vs pending review."
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [linux, macos]
category: git
category_taxonomy: git/dual-state-merge-review
date: 2026-09-25
metadata:
    hermes:
        tags: [git, merge, squash, review, staging, branches, aphrodite]
        related_skills: [git-operations, git-feat-dev-workflow]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
        - Current
---

# Dual-State Merge Review

Review a source branch's changes against a target branch WITHOUT committing:
the index carries the source's proposal (what the merge would do), the working
tree carries the target's corrections (what you actually want), and a human
walks every file pair. Built for reverse-direction merges where the source is
behind/divergent and its delta is mostly deletions or regressions.

## The five phases

1. **Scan & classify** (no mutations): fetch, merge-base, divergence counts;
   classify source-only commits (useful / destructive / already-ported); build
   the per-file verdict map. Remember: GitHub PR three-dot diffs show the
   MERGE-BASE as "before", not the target tip.
2. **Squash staging** (the proposal): `git merge --squash <source>` on the
   target branch - stages the full delta, no commit.
3. **Resolve conflicts manually, one file at a time** (the Conflict-state
   wave of the matrix below): read -> verdict -> `git add`. No scripts, no
   bulk, no restore/reset/checkout shortcuts - even an "obvious" exclusion
   gets a per-file read + verdict. Submodule conflicts resolve at the
   superproject gitlink; rename/delete pairs on the NEW path.
4. **Dual-state corrections** (index untouched, corrections unstaged):
   KEEP = leave; CORRECT = write target's HEAD content (MM); RESTORE a staged
   deletion = recreate from `git show HEAD:<path>` (D + ??); CONFIRM deletion =
   leave staged-only (D); REMOVE = plain `rm` (MD). Every staged file and
   every worktree-dirty file also gets its own matrix row - no file is
   reviewed by bulk.
5. **Review & commit**: human resolves each pair (stage a restore = keep;
   leave staged-D = delete; add the MM change = corrected content wins).

## The per-file state matrix (mandatory granularity)

Every file touched by the merge is reviewed THREE TIMES - once per git
state it can occupy - and each review is a full read, never a bulk pass.
One matrix per file, verdicts recorded before any `git add`:

| File | State(s) | merge-base | HEAD (ours) | Source (theirs) | staged | worktree | Verdict | Rationale |
| ---- | -------- | ---------- | ----------- | --------------- | ------ | -------- | ------- | --------- |

Review in three waves, one file at a time, in this order:

1. **Conflict-state wave** (`git diff --name-only --diff-filter=U`): for
   each unmerged file, read the base/ours/theirs stages
   (`git show :1/:2/:3:<path>`), author the merged content manually (or
   choose one side after reading BOTH), `git add` that file. Repeat until
   `--diff-filter=U` is empty. A repeated verdict (e.g. an excluded
   directory) is still applied per file - one read, one verdict, one add,
   never `git rm -rf <dir>` or `git checkout <side> -- <dir>`.
2. **Staged-state wave** (`git diff --cached --name-only`): for each staged
   file, read the staged blob vs HEAD vs Source; verdict (KEEP staged /
   CORRECT to HEAD / RESTORE deletion / CONFIRM deletion / rewrite).
3. **Worktree-state wave** (`git status --short` MM/M/?? entries): for each
   dirty file, read the worktree content vs the staged/HEAD versions;
   verdict (KEEP as correction / stage it / discard / leave untracked).

A file can appear in multiple waves (MM = staged AND worktree states) - it
gets a row per wave. The matrix is the merge record: any future question
("was X reviewed?") is answered by the row, not by memory.

## Pitfalls

- NEVER `git reset` the staged proposal mid-review (the dual view depends on
  the index). Deliberate abort: `git reset --hard HEAD` (squash has no
  MERGE_HEAD; `git merge --abort` won't work).
- Auto-committer risk: a conflicted index can't be auto-committed, but a blind
  `git add -A` sweep could stage one side - finish or stand down.
- **Bulk-resolution shortcuts are forbidden even when the verdict is
  uniform**: `git checkout <tree> -- <dir>`, `git restore --staged --worktree
-- <dir>`, `git rm -rf <dir>`, and scripted per-dir sweeps all skip the
  read-per-file requirement and silently drop the other side's content (the
  1.6.4 Finalize `environment:` gap class of bug). Exclusions included: a
  `.hermes/` exclusion is 42 per-file verdicts, not one `git rm -rf`.
- `write_file`/`patch` strip trailing newlines: byte-faithful restores need
  raw blob bytes (verify with `cmp`), else append `\n` and re-verify.
- A source branch's "test-free" deletions usually delete files the target
  actively uses (CI jobs, docs references) - check consumers first.
- A wholesale side-take can miss what the OTHER side added after the
  merge-base: diff the taken blob against BOTH sides, never just against
  the side you took.

## Artifact checklist

- [ ] 0 unmerged paths, 0 conflict markers in tracked files
- [ ] Staged = proposal set; pending = corrections
- [ ] Per-file state matrix complete: every conflict-, staged-, and
      worktree-state file has a row with a verdict and rationale
- [ ] Every restored file byte-identical to HEAD (`cmp`)
- [ ] Per-file ledger of decisions recorded

## References

- `references/worked-example.md` - full Current -> Development run (2026-09-18):
  commit classification, conflict ledger, dual-state decisions, tool pitfalls.
