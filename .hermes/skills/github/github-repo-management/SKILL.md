---
name: github-repo-management
description: "Use when cloning, creating, or forking GitHub repos; manage remotes, releases, secrets, CI for the PlayForm org and public repos."
version: 1.4.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: github
category_taxonomy: github/github-repo-management
date: 2026-09-25
metadata:
    hermes:
        tags: [github, repositories, git, releases, secrets, configuration]
        related_skills: [github-auth, github-actions-maintenance]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
        - Current
owns:
    - Repo-level CRUD and configuration flows (gh and REST)
    - The detach-fork procedure
depends_on:
    - github-auth
supersedes: []
verification:
    source_of_truth: Live GitHub API responses (`gh api` / `curl https://api.github.com`) and `git ls-remote` output; this file is not ground truth.
    mutation_level: high
---

# GitHub Repository Management

Create, clone, fork, configure, and manage GitHub repositories. Each section shows the `gh` command first. The `git` + `curl` fallback for each operation lives in `references/curl-fallbacks.md`; the full REST endpoint list lives in `references/github-api-cheatsheet.md`.

## When to Use

- User asks to clone, create, fork, configure, or manage a GitHub repository
- User needs repo settings, branch protection, secrets, releases, or CI runs
- User needs a `gh`-vs-`curl` fallback pattern for a repo-level GitHub API call

Aphrodite context: the PlayForm org repos (`PlayForm/Aphrodite`) default to the `Development` branch; `gh repo view PlayForm/Aphrodite` shows the default branch. Use `Development` wherever examples below say `main`, and protect `Development` (and `Current`) rather than `main` when configuring branch protection. The Aphrodite repo's canonical remote is named `Source` (`ssh git@github.com/PlayForm/Aphrodite.git`) and holds three submodules (`plugins/aphrodite`, `vendor/headroom`, `vendor/rtk`). Where examples read `origin`, substitute the actual remote name (`Source` or the clone remote).

## Prerequisites

- Authenticated with GitHub (see the `github-auth` skill)

### Setup

The setup script picks the transport. `AUTH` becomes `gh` only when the `gh` binary is present and `gh auth status` succeeds; otherwise `AUTH` becomes `git` and the script reads `GITHUB_TOKEN` from `~/.hermes/.env`, then from `~/.git-credentials`.

```bash
if command -v gh &> /dev/null && gh auth status &> /dev/null; then
	AUTH="gh"
else
	AUTH="git"
	if [ -z "$GITHUB_TOKEN" ]; then
		if [ -f ~/.hermes/.env ] && grep -q "^GITHUB_TOKEN=" ~/.hermes/.env; then
			GITHUB_TOKEN=$(grep "^GITHUB_TOKEN=" ~/.hermes/.env | head -1 | cut -d= -f2 | tr -d '\n\r')
		elif grep -q "github.com" ~/.git-credentials 2> /dev/null; then
			GITHUB_TOKEN=$(grep "github.com" ~/.git-credentials 2> /dev/null | head -1 | sed 's|https://[^:]*:\([^@]*\)@.*|\1|')
		fi
	fi
fi

# Get your GitHub username (needed for several operations)
if [ "$AUTH" = "gh" ]; then
	GH_USER=$(gh api user --jq '.login')
else
	GH_USER=$(curl -s -H "Authorization: token ***" https://api.github.com/user | python3 -c "import sys,json; print(json.load(sys.stdin)['login'])")
fi
```

If you are inside a repo already:

```bash
REMOTE_URL=$(git remote get-url origin)
OWNER_REPO=$(echo "$REMOTE_URL" | sed -E 's|.*github\.com[:/]||; s|\.git$||')
OWNER=$(echo "$OWNER_REPO" | cut -d/ -f1)
REPO=$(echo "$OWNER_REPO" | cut -d/ -f2)
```

Alternatively, source the shared auth helper from `github-auth`, which sets `$GITHUB_TOKEN`, `$GH_OWNER`, and `$GH_REPO`:

`source "${HERMES_HOME:-$HOME/.hermes}/skills/github/github-auth/scripts/gh-env.sh"`

Adapt `$OWNER`/`$REPO` to `$GH_OWNER`/`$GH_REPO` if you use it.

## 1. Cloning Repositories

Cloning is pure `git`; it behaves identically with or without `gh`.

```bash
# Clone via HTTPS (works with credential helper or token-embedded URL)
git clone https://github.com/owner/repo-name.git

# Clone into a specific directory
git clone https://github.com/owner/repo-name.git ./my-local-dir

# Shallow clone (faster for large repos)
git clone --depth 1 https://github.com/owner/repo-name.git

# Clone a specific branch
git clone --branch develop https://github.com/owner/repo-name.git

# Clone via SSH (if SSH is configured)
git clone git@github.com:owner/repo-name.git
```

With `gh` (shorthand):

```bash
gh repo clone owner/repo-name
gh repo clone owner/repo-name -- --depth 1
```

## 2. Creating Repositories

With `gh`:

```bash
# Create a public repo and clone it
gh repo create my-new-project --public --clone

# Private, with description and license
gh repo create my-new-project --private --description "A useful tool" --license MIT --clone

# Under an organization
gh repo create my-org/my-new-project --public --clone

# From existing local directory
cd /path/to/existing/project
gh repo create my-project --source . --public --push
```

The `git` + `curl` fallback for user, org, and template creation is in `references/curl-fallbacks.md`.

### From a Template

With `gh`:

```bash
gh repo create my-new-app --template owner/template-repo --public --clone
```

The curl `generate` flow is in `references/curl-fallbacks.md`.

## 3. Forking Repositories

With `gh`:

```bash
gh repo fork owner/repo-name --clone
```

The curl fork flow is in `references/curl-fallbacks.md`.

### Keeping a Fork in Sync

```bash
# Pure git - works everywhere
git fetch upstream
git checkout Development
git merge upstream/Development
git push origin Development
```

With `gh` (shortcut):

```bash
gh repo sync $GH_USER/repo-name
```

## 4. Repository Information

With `gh`:

```bash
gh repo view owner/repo-name
gh repo list --limit 20
gh search repos "machine learning" --language python --sort stars
```

The curl + python3 formatting variants are in `references/curl-fallbacks.md`.

## 5. Repository Settings

With `gh`:

```bash
gh repo edit --description "Updated description" --visibility public
gh repo edit --enable-wiki=false --enable-issues=true
gh repo edit --default-branch Development
gh repo edit --add-topic "machine-learning,python"
gh repo edit --enable-auto-merge
```

The curl PATCH and topics flows are in `references/curl-fallbacks.md`.

## 6. Branch Protection

There is no `gh` subcommand for branch protection; the API is the only path.

Never PUT branch protection without reading the current protection first, because the PUT replaces the whole config.

```bash
# View current protection
curl -s \
	-H "Authorization: token ***" \
	https://api.github.com/repos/$OWNER/$REPO/branches/Development/protection

# Set up branch protection
curl -s -X PUT \
	-H "Authorization: token ***" \
	https://api.github.com/repos/$OWNER/$REPO/branches/Development/protection \
	-d '{
    "required_status_checks": {
      "strict": true,
      "contexts": ["ci/test", "ci/lint"]
    },
    "enforce_admins": false,
    "required_pull_request_reviews": {
      "required_approving_review_count": 1
    },
    "restrictions": null
  }'
```

For the Aphrodite repos, protect `Development` (and `Current` where a release line is protected); `Build.yml` provides the required check context (`gh workflow list` shows the workflow name).

## 7. Secrets Management (GitHub Actions)

With `gh`:

```bash
gh secret set API_KEY --body "your-secret-value"
gh secret set SSH_KEY < ~/.ssh/id_rsa
gh secret list
gh secret delete API_KEY
```

Never set secrets through the REST path when `gh` is available, because the REST route requires PyNaCl encryption of the value and is easy to get wrong. If setting secrets is needed and `gh` is not available, install `gh` for just that operation. The encrypt-and-PUT curl flow is in `references/curl-fallbacks.md`.

## 8. Releases

With `gh`:

```bash
gh release create v1.0.0 --title "v1.0.0" --generate-notes
gh release create v2.0.0-rc1 --draft --prerelease --generate-notes
gh release create v1.0.0 ./dist/binary --title "v1.0.0" --notes "Release notes"
gh release list
gh release download v1.0.0 --dir ./downloads
```

Aphrodite-specific releases (crate versions, the plugin binary, the `BINARY_VERSION` pairing, publish steps) are owned by `aphrodite-release-workflow`. Never use these generic `gh release` commands to override the release ceremony for the Aphrodite crates, because the ceremony enforces the crate/version pairing. Use the generic commands only for other repos or ad-hoc operations. The curl create/list/asset-upload flows are in `references/curl-fallbacks.md`.

## 9. GitHub Actions Workflows

With `gh`:

```bash
gh workflow list
gh run list --limit 10
gh run view <RUN_ID>
gh run view <RUN_ID> --log-failed
gh run rerun <RUN_ID>
gh run rerun <RUN_ID> --failed
gh workflow run Build.yml --ref Development
gh workflow run Publish.yml -f environment=staging
```

The curl list/logs/rerun/dispatch flows are in `references/curl-fallbacks.md`.

## 10. Gists

With `gh`:

```bash
gh gist create script.py --public --desc "Useful script"
gh gist list
```

The curl create/list flows are in `references/curl-fallbacks.md`.

## Detaching Forks (making a fork standalone + private)

GitHub has no API to un-fork a repo. A public fork cannot be made private directly; the PATCH fails with "Public forks can't be made private". The only automated path is delete+recreate:

1. Export metadata first, before deleting: `gh api --paginate repos/{o}/{r}/pulls?state=all` plus the issues and comments JSON. This export is the only copy of the PR titles and bodies once the fork is gone.
2. Mirror clone for a full backup, including `refs/pull/*`: `git clone --mirror URL`. Keep the mirror until step 6 passes.
3. Delete the fork: `gh api -X DELETE repos/{o}/{r}`. This requires the `delete_repo` OAuth scope; without it the call returns 403. Add the scope with `gh auth refresh -h github.com -s delete_repo`. The device flow needs the user to enter the one-time code, and the flow does not start polling until Enter is pressed.
4. Recreate the same name as private: `gh api -X POST orgs/{o}/repos -f name=... -f private=true -f description=... -f has_issues=... -f has_wiki=...`. Never include `has_projects`, because orgs with the Projects feature disabled reject the field even as false. Repo creation is secondary-rate-limited; space creations out (90s+) and retry with backoff.
5. Push all refs: `git --git-dir=mirror push URL '+refs/heads/*:refs/heads/*' '+refs/tags/*:refs/tags/*'`. Pushing branches that contain `.github/workflows/*` requires the `workflow` OAuth scope; without it the push fails with "refusing to allow an OAuth App to create or update workflow". Never use `push --mirror`, because GitHub rejects `refs/pull/*`.
6. Verify refs: compare `git ls-remote` (filter out `^{}` peeled tag lines) against the mirror's `for-each-ref --format='%(refname) %(objectname)'`. The diff must be empty.
7. Restore the default branch if it is not main/master (PATCH `default_branch`). Recreate PRs from the export; same-repo PRs need their head branches pushed. Merged or closed PRs with deleted branches are unrecreatable, and their diff lives only in history.

Gotchas: never re-run a detach script that re-exports after deletion, because it overwrites the good export with a 404 body; export once, pre-delete. Archived orgs cannot be unarchived via API, and repos in them stay public. GitHub's Dependabot security-updates may auto-recreate equivalent dependency PRs on the fresh repos.

## Stop if / Recovery

Stop if the fork has not been exported (`gh api --paginate repos/{o}/{r}/pulls?state=all`) and mirror-cloned (`git clone --mirror URL`) before a delete; if a detach script would re-export after deletion; if you are about to `push --mirror`; or if the current branch protection has not been read (`curl -s -H "Authorization: token ***" https://api.github.com/repos/$OWNER/$REPO/branches/Development/protection`).

Recovery: the only legal next act is to run the missing read first - export and mirror before any delete, GET protection before any PUT, and `'+refs/heads/*:refs/heads/*' '+refs/tags/*:refs/tags/*'` in place of `push --mirror`. If a 404 body already overwrote the export, the mirror is the only remaining backup.

## Pitfalls

- Never set a secret through the REST path when `gh` is available, because the REST route requires PyNaCl encryption of the value and is easy to get wrong.
- Never clone full history for inspection, because `--depth 1` shallow clones are faster for large repos and full history is rarely needed.
- Never PUT branch protection without reading the current protection first (`curl -s -H "Authorization: token ***" https://api.github.com/repos/$OWNER/$REPO/branches/Development/protection`), because the PUT replaces the whole config.
- Never paste a real token into a command, because it leaks into shell history and logs. Keep the redacted `Authorization: token ***` placeholder in every curl example.

## Claim-to-test table

| Claim                                                                                        | Test                                                                                                                                                                |
| -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `AUTH` resolves to `gh` when the binary is present and authenticated                         | `gh auth status` exits 0                                                                                                                                            |
| `gh repo create my-new-project --public --clone` creates and clones a repo                   | `gh repo view my-new-project` exits 0 and `test -d my-new-project` succeeds                                                                                         |
| `gh repo create my-project --source . --public --push` publishes an existing local directory | `git ls-remote` on the new remote shows the pushed head                                                                                                             |
| `gh repo sync $GH_USER/repo-name` brings a fork up to date                                   | `git fetch upstream` then `git log --oneline origin/Development..upstream/Development` returns empty output                                                         |
| The branch protection PUT replaces the whole config                                          | `curl -s -H "Authorization: token ***" https://api.github.com/repos/$OWNER/$REPO/branches/Development/protection` shows only the fields from the last PUT           |
| `gh secret set API_KEY --body "your-secret-value"` writes a secret                           | `gh secret list` shows `API_KEY`                                                                                                                                    |
| `gh release create v1.0.0 --title "v1.0.0" --generate-notes` publishes a release             | `gh release list` shows `v1.0.0`                                                                                                                                    |
| `gh workflow run Build.yml --ref Development` triggers the workflow                          | `gh run list --limit 10` shows a run for `Build.yml`                                                                                                                |
| A public fork cannot be made private directly                                                | the PATCH to `/repos/{owner}/{repo}` with `"visibility": "private"` fails with "Public forks can't be made private"                                                 |
| The detach-fork push restores all refs                                                       | `git ls-remote` on the recreated repo compared with the mirror's `for-each-ref --format='%(refname) %(objectname)'` shows an empty diff after filtering `^{}` lines |
