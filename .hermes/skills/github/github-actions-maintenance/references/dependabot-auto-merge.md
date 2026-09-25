# Dependabot auto-merge diagnosis and org-wide branch protection

## The failure signature

Merge job in a Dependabot auto-accept workflow fails with:

```
GraphQL: Pull request Protected branch rules not configured for this branch (enablePullRequestAutoMerge)
```

This is GitHub rejecting the `enablePullRequestAutoMerge` GraphQL mutation (what `gh pr merge --auto` calls). Precondition: the base branch must have branch protection rules requiring at least one status check OR review. No rules → refuse.

## Why single-PR batches work and multi-PR batches fail

gh CLI (`pkg/cmd/pr/merge/merge.go`) computes:

```go
autoMerge: opts.AutoMergeEnable && !isImmediatelyMergeable(pr.MergeStateStatus)
```

`isImmediatelyMergeable` returns true only for CLEAN, HAS_HOOKS, UNSTABLE. When true, gh merges directly (plain merge mutation — no protection needed, silent in non-TTY). When false (BEHIND, BLOCKED, ...), gh calls `enablePullRequestAutoMerge`.

Batch collision: dependabot opens N PRs at once. The 1st PR's Merge job runs while the PR is still CLEAN/UNSTABLE → direct merge, succeeds. The 2nd/3rd PRs' Merge jobs run seconds later, after the base moved → their head is BEHIND → auto-merge path → GitHub refuses (no protection) → Merge job fails, PR left open and approved.

## Diagnosis steps

1. Confirm the workflow DID trigger: `gh api repos/{o}/{r}/actions/runs?per_page=100` — filter `name=Dependabot` + `event=pull_request`. A "didn't trigger" report is often actually a triggered run whose Merge job failed.
2. Read the failing Merge job log: `gh api repos/{o}/{r}/actions/runs/{id}/jobs` → job id → `gh api repos/{o}/{r}/actions/jobs/{jid}/logs` — grep for `GraphQL:`.
3. Confirm branch protection state: `gh api repos/{o}/{r}/branches/{b}/protection` → `404 Branch not protected`; repo rulesets: `gh api repos/{o}/{r}/rulesets`; org rulesets need a paid plan (`403 Upgrade to GitHub Team` on free orgs).
4. Compare a successful sibling run from the same batch — same workflow file, same runner image → the variable is repo state (base moved), not the workflow.

## Org-wide apply recipe

Enumerate: `gh api --paginate user/orgs --jq '.[].login'`, then per org `gh api --paginate "orgs/$org/repos?per_page=100"` capturing `default_branch`, `archived`, `fork`. Reconcile per-org sums against the total file line count — silent truncation loses stragglers.

Per repo (script loop):

1. Skip archived repos.
2. Skip repos already protected (`GET branches/{b}/protection` returns 200) — never overwrite existing rules.
3. Detect `.github/workflows/GitHub.yml` on the default branch: present → required check `Assign` (strict=false); absent → required review count 1.
4. `PATCH repos/{o}/{r}` with `allow_auto_merge=true`.
5. PUT the protection body from a JSON file:

```json
{
	"required_status_checks": { "strict": false, "contexts": ["Assign"] },
	"enforce_admins": false,
	"required_pull_request_reviews": null,
	"restrictions": null
}
```

```bash
gh api -X PUT repos/{o}/{r}/branches/{b}/protection --input body.json
```

`gh api -f` stringifies booleans/ints (422 on `strict`); `-F`/`-f` mixes still 422 on nested `required_pull_request_reviews`. The JSON file is the reliable path.

Run as a background script with `notify_on_complete=true`; log every repo `applied/skipped/failed`; verify with a read-back pass.
