---
name: shell-script-conventions
description: "Use when editing shell scripts in the Aphrodite workspace or the user's private rc files. Escape commands against aliases, keep scripts pure ASCII and shfmt-clean."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [linux, macos]
category: engineering
category_taxonomy: engineering/shell-script-conventions
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, escaping, verifying, shfmt-ing, ascii-checking]
        related_skills: [github-actions-maintenance]
status: active
---

# Shell Script Conventions (repo and user-owned scripts)

Governs every edit to shell scripts in the Aphrodite workspace: repo helper
scripts (`scripts/`, the `Build.yml`/`Publish.yml`-driven wrappers, plugin
`install.sh`-style scripts), the user's private rc files (the private
environment file, `.zshrc`, `.bashrc` - kept out of the public tree), and
any wrapper the user owns. Class-level: the standing convention applies to
ALL of them, every time, without asking.

## Always-on rules

1. **Backslash-escape command invocations in every owned shell script** -
   `\cp`, `\echo`, and any other command that could be aliased - at EVERY
   call site, including mid-line forms (`$(\echo ...)`, `|| \echo ...`,
   `; \echo ...`), not just line starts. The standing convention is `\cp`
   everywhere; rewrite all call sites.
2. **Never modify or remove the user's aliases** (e.g. `alias cp='cp -i'` in
   `.bashrc` or an oh-my-zsh common-aliases plugin). The escapes in scripts
   are the fix, aliases are the user's own and stay exactly as they are.
3. **Scope: repo/user-authored files only.** Vendored/third-party content is
   left untouched: `vendor/headroom` build scripts, oh-my-zsh
   themes/plugins, `.bash-preexec.sh`, upstream submodule scripts. If unsure
   whether a file is authored in-repo, check git history/ownership before
   editing.
4. **Explicit env-var overrides resolve BEFORE `command -v` PATH discovery
   in wrapper scripts.** A wrapper that can point at a specific tool instance
   (e.g. `AUTH_CLOUDFLARE_BIN`) must read the override first and fall back
   to PATH only when it is empty - resolving PATH first lets a STALE
   installed binary silently shadow the override and regenerate stale output
   while the script exits 0. The explicit override wins, always.

## Why: alias expansion happens at PARSE time

zsh and bash expand aliases when a line is _parsed_, not when it runs - a
function body defined while an alias is active has the alias expansion baked
in permanently. A rc file that re-sources another file AFTER the shell
loaded aliases (e.g. `.zshrc` re-sourcing the private environment file after
oh-my-zsh) silently turns every `cp -f` inside the sourced file's functions
into `cp -i -f` → interactive "overwrite? (y/n)" prompts inside automated
builds (symptom: a build "hangs waiting for Enter" twice per run). `\cp`
bypasses alias expansion at parse time - the only reliable fix.

A `zsh -c` probe CANNOT reproduce this - non-interactive zsh does not load
`.zshrc`. Reproduce/verify with a PTY probe running `zsh -i` and feeding no
input: if the chain completes without prompting, the escapes work.

## Editing procedure (avoid the backslash-doubling footgun)

1. Escape **by form, one pattern per pass**, never a blanket `echo` →
   `\echo` replace_all: `echo "` → `\echo "`, then `echo '` → `\echo '`,
   then `cp "` → `\cp "`, then any remaining bare `echo`/`cp` lines. A
   blanket substring replace matches `echo` INSIDE already-escaped `\echo`
   lines and doubles them (`\\echo`) - this bit repeatedly.
2. After every pass, un-double: replace_all `\\echo` → `\echo` - but then
   re-verify with a fresh read.
3. **Verify with read_file (or `od -c` on one line), never grep.** Grep
   backslash patterns are self-trapping: BRE `\` matches one literal
   backslash, PCRE `\\` two, and shell quoting layers it again - the same
   command misreads single as doubled and vice versa. `read_file` shows the
   file's true bytes.
4. **The patch tool doubles backslashes in `new_string` for shell files.**
   For backslash-heavy rewrites (a whole canonical script), use `write_file`
   with the full intended content instead of patching, then `bash -n`.
5. **Patches can land full-width / non-ASCII lookalike characters in ASCII
   shell scripts - `bash -n` does NOT catch them; verify byte purity.** A
   full-width `）` where an ASCII `)` belongs (or a smart quote replacing
   `'`) parses fine under `bash -n`/`shfmt` and is invisible in a normal
   diff read; the script fails or misbehaves only when something actually
   executes the token. Shell scripts must stay pure ASCII (they run under
   git-bash on Windows too). After any patch to a `.sh`/`.ps1`, scan for
   non-ASCII bytes: `grep -P '[^\x00-\x7F]' <file>` (on macOS `od -c
<file>` - `cat -A` is not a valid option there) and re-patch any hit to
   its ASCII equivalent. This is a composition error the patch tool applies
   faithfully - fix the exact line, do not debug the logic.

## shfmt owns `.sh` - prettier must NOT touch shell files

The repo's `.prettierignore` explicitly delegates shell: `# Shell - shfmt
owns .sh / .bash` plus `*.sh`. An editor's prettier therefore logs
"ignored, skipping" + `inferredParser: null` for every .sh file - that is BY
DESIGN (the prettier sh plugin is deliberately not wired for shell). Do NOT
"fix" it by removing `*.sh` from `.prettierignore` or adding a prettier sh
plugin - that contradicts the repo design. Format touched .sh files with the
designated tool: `shfmt -w <file>` (locate with `command -v shfmt`), then
verify `shfmt -d <file>` reports zero diff. shfmt leaves `\echo`/`\cp`
escapes intact - it only normalizes indentation/quoting/redirection spacing.

## Verification battery (run before declaring done)

- `bash -n` (and `zsh -n` for rc files) on every edited file.
- Grep sweep for bare `^\s*(echo|cp)` across every edited tree - zero
  matches.
- Grep sweep for doubled `\\echo`/`\\cp` via `grep -rPn '\\\\echo|\\\\cp'`
  (PCRE: four shell-quoted backslashes = two literal) - zero matches.
- If an interactive-alias regression is suspected, run the PTY probe (above)
  in the real shell the user launches.
- `shfmt -d` zero diff on every touched .sh file.
- Leave edits uncommitted - the user's auto-committer/Save flow sweeps them.
