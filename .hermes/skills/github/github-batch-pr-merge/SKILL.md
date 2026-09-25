---
name: github-batch-pr-merge
description: "Use when bulk-merging or closing open PRs across repos/orgs. Squash-merge sweeps with conflict-close classification for the PlayForm org."
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: github
category_taxonomy: github/github-batch-pr-merge
date: 2026-09-25
metadata:
    hermes:
        tags: [github, pull-requests, automation, merge, batch]
        related_skills: [github-pr-workflow, github-auth]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
        - Current
owns:
    - The enumerate + sweep procedure (org-wide and single-repo)
    - The merge-conflict close classification
depends_on:
    - github-auth
supersedes: []
---

# GitHub Batch PR Merge / Close Sweep

Merge every open PR in one pass across a repo or an entire org; close only
the ones that actually conflict. Verified on runs of 10-250 dependabot PRs
(single repos and org-wide fork farms - including PlayForm org sweeps). For
single-PR lifecycle (branch, open, CI, one merge) use the bundled
`github-pr-workflow` skill - this skill is the bulk-sweep class.

## When to Use

- "Merge all pull requests" / "do this for all repos in the org"
- Clearing a dependabot backlog across the PlayForm fork-farm org

## Prerequisites

- `gh` authenticated with `repo` scope - see `github-auth`

## Procedure

1. **Enumerate** - `scripts/enumerate-open-prs.sh <owner>` (org-wide) or
   `gh pr list --repo O/R --state open --limit 100 --json number,title,mergeable,mergeStateStatus,headRefName`
   (single repo).
    - Org-wide search flag is `--owner ORG`, never `--org` (unknown flag ->
      help text + exit 1).
    - Search caps at 100 results per query: confirm the real total with
      `search/issues` `.total_count` and paginate when truncated - a round
      count of 100 (or any page size) means "possibly truncated", not
      "exactly 100".
    - Save the list as `OWNER/REPO<TAB>NUMBER` lines in a scratch file under
      `~/.hermes/tmp/`.
2. **Run the sweep** - `scripts/merge-close-sweep.sh <list-file>`. Merges land
   on the repo's default branch (`Development` for PlayForm repos). Per PR:
    - `gh pr merge --squash --delete-branch`; exit 0 -> merged.
    - Error mentions "merge conflict" -> close with comment
      `Closed: merge conflicts with the base branch.`.
    - Any other error -> log and LEAVE OPEN (required checks, branch
      protection, archived repo - closing those destroys PRs the user
      wanted).
    - SUMMARY line (`merged=... closed_conflict=... failed=...`) must sum to
      the enumerated total.
3. **Run it backgrounded** (100+ PRs takes minutes, ~0.5-1 s per gh call):
   `background=true` + `notify_on_complete=true`, tee all lines to the log.
4. **Re-scan and verify** - dependabot opens new PRs during a long run;
   re-run the enumeration and process stragglers until `total_count` == 0.

## Pitfalls

- **Never trust the `mergeable` field for decisions.** `gh pr list` reports
  `UNKNOWN` for most PRs (GitHub computes mergeability lazily), and a PR
  that reports `MERGEABLE` can still fail at merge time with `Pull Request
has merge conflicts` (stale computation). The merge attempt is the only
  authoritative conflict check.
- **Parse `OWNER/REPO` + number with a tab separator, never `IFS=/`.**
  `read -r repo num` with `IFS=/` on `O/R/N` splits into `repo=O`,
  `num=R/N`, and every call dies with `expected the "[HOST/]OWNER/REPO"
format, got "O"` - a whole 234-PR batch can fail in one pass with zero
  work done. Emit tab-separated, read with `IFS=$'\t'`, and sanity-check
  the list (`awk -F'\t' '$1 !~ /^O\//'`) before launching.
- **Close only on a conflict error**, per the classification in the sweep
  script - conflict-checking by the merge attempt is the pattern, not a
  pre-pass.
- Verify the end state (count == 0) by enumeration, not by trusting the
  batch's exit code - the sweep script exits 0 even when every PR failed.
