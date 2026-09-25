# Cleanup discipline: worked recipes

Reference for the standing user rules in SKILL.md ("Cleanup discipline").
Each recipe is a worked instance of a rule; the rule itself lives in
SKILL.md. All commands are byte-stable.

## Repair-in-place versus git-restore (worked detail)

`git checkout -- <file>` restores from the INDEX, not HEAD. With an
auto-committer that stages working-tree edits, the checkout is a NO-OP: it
returns the staged version and the file still "contains your edits" - grep
finds the symbol, `git status` shows nothing.

When a file's CONTENT is wrong (e.g. escaped `\n` newlines), git-restoring
throws the intended work away - a corrupted write still contains that
content, and escaped newlines are recoverable data. Repair in place:

1. `read_file` the file.
2. Find the improper section.
3. Fix exactly that with `patch`/`write_file`, preserving every intended row.

If the working tree was ALREADY reset, recover the original blob from the
object store first:

```sh
git fsck --lost-found
git cat-file -s <hash>          # match by size
git cat-file blob <hash> > file # extract
```

then repair and re-write. When `git status` shows `MM` (index holds stale
text, working tree holds the fix), align the index with `git add` - a
forward action, never reset/checkout.

This rule governs files carrying intended content; the read-only mirror
fleet resets (nothing authored locally) are a different operation.

## Locating keep-able content before dropping a commit

Before dropping a commit as "test junk", locate the content you want to keep
with `git log --oneline -- <path>`. The auto-committer bundles hook
refactors/real fixes into unrelated-looking "bump"/"chore" commits - a
commit that looks droppable may be the ONLY carrier of keep-able content.
Verify what each commit in the drop range actually touches before removing
it.

## Scrubbing distributed/release branches (worked detail)

Agent dirs (`.hermes/`, `skills/`, agent-feedback docs, test markers like
`.hook-battery-test`) belong to the WORK branch only. The distributed branch
must be scrubbed before the first release and never re-imported by a
transplant.

Scrub = `git rm -r .hermes skills <agent-docs>`, strip `.gitignore`
negations that re-include dev dirs (`!.hermes/`), and commit the scrub ON
the distributed branch. Also pin LF for hook files in `.gitattributes`
(`.githooks/* text eol=lf`) while there - see the CRLF pitfall in
`gitlink-hooks.md`. Verify with `git ls-files | grep -E
'^(\\.hermes/|skills/)'` empty on the release branch.

### Heartbeat/auto-commit workflows must be retargeted in the same scrub commit

A cron workflow pushing `branch: Current` writes daily commits into the
distributed line and re-creates any removed heartbeat file
(`.github/Update.md`) on its next run, undoing the scrub. Change the
workflow's push target to the work branch in the same scrub commit.

### Remove dead references to the scrubbed paths, not just the files

- `workspace.exclude` entries naming removed crates.
- pre-commit hook blocks calling removed scripts - delete the whole
  `if [ -x ... ]` block, not just the path.
- `.gitignore` patterns pointing at removed dirs.
- `.gitattributes` `export-ignore` lines for removed dirs.

### Non-shipping experimental crates belong on the work branch only

Crates excluded from `workspace.members`, known-unbuildable features - their
removal from the distributed branch also removes their advisory vectors.

## Back up before you scrub; preserve what `git rm` cannot touch

The user wants removed dev material saved to scratch (`~/.hermes/tmp/`),
never deleted outright.

- Tracked files: `git archive HEAD <paths...> | tar -x -C <scratch-dir>`
  preserves the exact committed content pre-delete (tar's `-x` extraction,
  not the archive itself).
- Untracked dev dirs (`.plans/`, `.bench/`, build/results leftovers inside
  a removed tree) SURVIVE `git rm` and stay in the repo - `git rm` only
  removes tracked files. Physically `mv` them out:
  `mv .plans .bench bench <scratch>/moved-from-current/`.
- Then re-verify with `git status --porcelain` that nothing untracked
  remains and the tree is clean (or holds only the intended scrub commit).