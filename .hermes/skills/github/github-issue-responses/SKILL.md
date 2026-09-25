---
name: github-issue-responses
description: "Use when drafting public responses to GitHub issue reports. Verify claims in code and against shipped artifacts before replying."
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: github
category_taxonomy: github/github-issue-responses
date: 2026-09-25
metadata:
    hermes:
        tags: [github, issues, responses, bug-reports]
        related_skills: [github-issues, github-code-review]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
        - Current
owns:
    - The verify-in-code-before-replying gate
    - The shipped-artifact version check
depends_on:
    - github-issues
    - github-code-review
supersedes: []
---

# GitHub issue responses

Authoring public replies to bug reports on the PlayForm org repos (and
upstream contributions): confirming or denying the report, correcting a
wrong root-cause theory, announcing where fixes landed.

## When to Use

- A maintainer asks "how to respond to this issue" or hands you a report
  to draft a reply for.
- You need to state what a released version does/doesn't do in a public
  channel.

## Procedure

1. **Verify the claim in the CODE, not the report.** Trace the reported
   path in the current source (for Aphrodite: the proxy crate
   `crates/aphrodite`, the Hermes bridge `crates/aphrodite-hermes`, or the
   plugin `plugins/aphrodite`); the reporter's trigger/symptom can be
   real while their root-cause theory is wrong - say so explicitly and
   name where the reported symptom actually lives.
2. **Verify any version claim against the SHIPPED artifact**: registry
   max version (crates.io sparse index for the `aphrodite` /
   `aphrodite-hermes` crates) AND the release commit's manifest - never
   the dev workspace. A `Development` branch carries newer/different deps
   than what release users resolve; a dev-workspace claim gets challenged
   and the correction is a public round-trip. For the plugin, pair the
   installed binary with its `BINARY_VERSION` instead of assuming the
   workspace state.
3. **Draft in the user's voice** (exemplar): a one-line verdict opener
   ("This is a real bug."), evidence-first ("We tested ...") before
   explanation, a bulleted fix list in backticks, no hedging. Match the
   repo's public-facing markdown conventions: GFM alert blocks (tag alone
   on the `>` line, blank `>`, then the body) for key callouts and labeled
   code fences for snippets.
4. **Hold the reply until explicitly instructed to post.** Draft to a
   scratch file under `~/.hermes/tmp/`; never comment/close from a draft.
5. **Credit the reporter**: `Fixes #N` in the fixing commit, release-notes
   lines, `Co-authored-by` for substantive contributions.

## Pitfalls

- **Accepting the reporter's root-cause theory as fact** - the expensive
  trap is fixing the described cause while the real defect sits in a
  sibling path. Trace first, then confirm or deny the theory by name.
- **Asserting "the shipped X has Y" from the dev workspace** - verify
  against the released artifact + release commit before writing it.
- **Rewriting when the user has already drafted their own reply** -
  correct nits (conflated connectives, grammar, inverted phrasing)
  instead; their voice wins.
