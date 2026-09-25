---
name: dual-state-merge-review
description: "Use when reviewing branch merges: staged vs pending review."
version: 1.1.0
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
3. **Resolve conflicts manually**, per file: read -> verdict -> `git add`.
   No scripts, no bulk, no restore/reset/checkout shortcuts. Submodule
   conflicts resolve at the superproject gitlink; rename/delete pairs on the
   NEW path.
4. **Dual-state corrections** (index untouched, corrections unstaged):
   KEEP = leave; CORRECT = write target's HEAD content (MM); RESTORE a staged
   deletion = recreate from `git show HEAD:<path>` (D + ??); CONFIRM deletion =
   leave staged-only (D); REMOVE = plain `rm` (MD).
5. **Review & commit**: human resolves each pair (stage a restore = keep;
   leave staged-D = delete; add the MM change = corrected content wins).

## Pitfalls

- NEVER `git reset` the staged proposal mid-review (the dual view depends on
  the index). Deliberate abort: `git reset --hard HEAD` (squash has no
  MERGE_HEAD; `git merge --abort` won't work).
- Auto-committer risk: a conflicted index can't be auto-committed, but a blind
  `git add -A` sweep could stage one side - finish or stand down.
- `write_file`/`patch` strip trailing newlines: byte-faithful restores need
  raw blob bytes (verify with `cmp`), else append `\n` and re-verify.
- A source branch's "test-free" deletions usually delete files the target
  actively uses (CI jobs, docs references) - check consumers first.

## Artifact checklist

- [ ] 0 unmerged paths, 0 conflict markers in tracked files
- [ ] Staged = proposal set; pending = corrections
- [ ] Every restored file byte-identical to HEAD (`cmp`)
- [ ] Per-file ledger of decisions recorded

## References

- `references/worked-example.md` - full Current -> Development run (2026-09-18):
  commit classification, conflict ledger, dual-state decisions, tool pitfalls.
