---
name: save-oneshot-tooling
description: "Use when debugging or modifying the Save gcommit oneshot commit-message tooling."
version: 0.1.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: git
category_taxonomy: git/save-oneshot-tooling
date: 2026-09-25
metadata:
    hermes:
        tags: [git, save, gcommit, oneshot, commit-message, hermes-wrapper]
        related_skills: [git-operations]
status: active
---

# Save / gcommit Oneshot Tooling

**Class:** the local commit-message generation stack: `Save` (Rust binary),
`HermesOneshotWrapper.py` (Python), `Ask` prompt files, and the `git gcommit*` aliases that
wire them to Hermes oneshot inference.

## Architecture (read before touching anything)

1. `git gcommit-hermes` runs the alias from `~/.gitconfig`:
   `<workspace>/Save/Target/release/Save --hermes
--system-prompt-file <workspace>/Ask/Prompt/Save/Message.md --thinking`
2. Save (Rust, `Save/Source/Library.rs`) spawns `HermesOneshotWrapper.py` with a baked
   `--model deepseek-v4-flash` and the prompt in a temp file (never on the command line).
3. The wrapper reads the prompt, rewrites the model, builds a Hermes `AIAgent` with
   `session_db=None` (no history pollution), and calls `agent.chat(prompt)`.

Key mechanics:

- The Save binary **hardcodes the wrapper filename** (joins `"Fn/HermesOneshotWrapper.py"`
  onto CARGO_MANIFEST_DIR's parent at compile time) - a git alias cannot redirect it; the
  wrapper is the only place to patch behavior without rebuilding Rust.
- The binary does **not** call `env_clear()` - the child inherits the environment, so env
  vars (e.g. `SAVE_ONESHOT_BACKEND`) are a valid control channel.
- `SAVE_ONESHOT_BACKEND=cline` makes the wrapper `os.execv` `HermesOneshotWrapperCline.py`,
  which strips `--model` and applies its own default. The fixed-model rule below applies to
  the default path only.
- `<workspace>` may be reachable through more than one path (symlinked roots) - the two
  paths are THE SAME files. Check with `readlink` before assuming two copies exist.

## Fixed model/provider rule (standing user preference)

The default oneshot path must ALWAYS force:

- `FORCE_MODEL = "@cf/deepseek-ai/deepseek-v4-flash-0731"`
- `FORCE_PROVIDER = "custom:cloudflare"`

Never inherit the `~/.hermes/config.yaml` default model/provider for the oneshot. The user
chose this model as cheap and reliable for commit messages.

## Lesson: a model id only exists on its owning endpoint

A model override without a provider pin falls back to the config default provider, and the
id goes to the wrong endpoint (e.g. `tencent/hy3:free` → Cloudflare rejects it with
`HTTP 400: No such model ... code 5007`). When a wrapper/script overrides a model, pin the
provider in the same edit - otherwise the config default wins and the id is sent to the
wrong endpoint.

## Debug technique: replicate the routing before any API call

To see exactly where a model would be sent, run the same resolution the wrapper runs, using
the Hermes venv python (match the venv's python version in the site-packages path, e.g.
`lib/python3.11`):

```sh
~/.hermes/hermes-agent/venv/bin/python3 - << 'EOF'
import sys, os
sys.path.insert(0, os.path.expanduser("~/.hermes/hermes-agent/venv/lib/python3.11/site-packages"))
from hermes_cli.config import load_config
from hermes_cli.models import detect_provider_for_model
from hermes_cli.runtime_provider import resolve_runtime_provider
cfg = load_config()
print("detected:", detect_provider_for_model("SOME_MODEL", "custom:cloudflare"))
runtime = resolve_runtime_provider(requested=None, target_model="SOME_MODEL")
print({k: v for k, v in runtime.items() if k not in ("api_key", "credential_pool")})
EOF
```

- `requested=None` reproduces the config-default fallback (what happens when no provider is
  pinned); `requested="custom:cloudflare"` reproduces a pinned provider.
- The printed `base_url` + `model` pair is the decisive fact: if they don't belong together,
  the call will fail with "No such model".
- hermes_cli is an editable install - its source lives at
  `~/.hermes/hermes-agent/hermes_cli/`; the site-packages dir only holds a `.pth` finder.
  Read the source there when you need the ladder logic.

## Verification after a wrapper edit

1. `python3 -m py_compile <wrapper>` - syntax clean.
2. `grep -n "FORCE_MODEL\|tencent/hy3" <wrapper>` - no stale force-model leftovers.
3. Resolution trace shows `base_url=api.cloudflare.com/.../ai/v1` with
   `model=@cf/deepseek-ai/deepseek-v4-flash-0731`.
4. End-to-end: run `git gcommit-hermes` in a repo with staged changes; expect
   `[Save] [INFO]: Committed successfully.` and `EXIT:0`.

## Pitfall: no staged changes = silent no-op, and auto-committers sweep first

`gcommit-hermes` (and `gcommit`/`ecommit`) does NOT stage anything. With an empty
index it prints `[Save] [INFO]: No staged changes found.` and exits 0 - no error,
no commit. Before invoking: `git add` explicitly, and check `git status --short`
FIRST because an external auto-committer may have already swept the working tree
into a commit with a generic message (in that case `git log -1` shows the sweep
commit on top and the wrapper is redundant). Do not conclude "the tool is broken"
from an empty log after a wrapper run - stage and re-run once, then verify with
`git log -1` that the commit landed.

## Pitfall: multi-repo commits - submodule FIRST, parent second

When committing a submodule pair (e.g. Aphrodite monorepo + `plugins/aphrodite`),
commit the SUBMODULE first, then the parent: the parent's `git add -A` picks up
nothing unless the submodule commit already moved the gitlink, so a parent-first
commit records the stale pointer and needs a second pointer-only commit. Verify
each landed with `git log -1` before moving to the next repo. The Save backend's
`warning: adding embedded git repository: plugins/aphrodite` stderr noise is
harmless - judge by the `[Development <sha>]` commit line, not stderr.

## Related skills

- `cloudflare-operations` - Workers AI endpoint/error table (code 5007 row)
- `git-operations` - general git repo maintenance
