# Release Notes - Content Standards + heredoc template

Owner: `aphrodite-release-workflow` (section 5 of the owning SKILL.md).
Canonical template: `.hermes/release/RELEASE-TEMPLATE.md` (defines **Live**
vs **Retrospective** modes - retrospective never claims to have rebuilt or
retested a shipped release).

Every release MUST include Summary, Changes, Infrastructure (live) or
Verification (retrospective), What Ships, and Links.

## Standards

- Drafts live in `.hermes/release-notes/` (`vNEXT-draft.md`,
  `headroom-fork-vNEXT-draft.md`). Stage the final notes file explicitly
  (`.hermes/release-notes/release-notes-vX.Y.Z.md`) - verify it is actually
  committed; it can drop out between `git add` and commit.
- **Live mode** (cutting the release now) requires a real `### Infrastructure`
  section with commands you actually ran. **Retrospective** (rewriting an
  already-shipped release) replaces Infrastructure with `### Verification`
  describing what was analyzed (commit range, diffstat) - never re-test.
- Draft placeholders (`{PENDING}` in Infrastructure, `{VERSION}` /
  `{PLUGIN_VERSION}` in title/compare link, `DO NOT PUBLISH` header) are
  by-design for drafts - never publish a note still containing `{PENDING}`.
- Headroom-fork notes are retrospective and separate from the binary notes;
  they use the fork's `aphrodite-vX.Y.Z` tag scheme and the
  `aphrodite-headroom-core` package name.
- **What Ships** lists the full fixed 4-target matrix (per
  RELEASE-TEMPLATE.md), never a point-in-time asset snapshot; never write
  "no Windows release" - the slow Windows leg is a timing race, and
  Build.yml's `Finalize` job fails the release if the matrix is incomplete.
- **Contributor credit:** co-authored work carries `Co-authored-by: Name
  <email>` trailers; issue-fixing changes reference `Fixes #N` in the change
  bullet or commit so the note links back to the issue.
- Never ship a bare compare link with zero description.
- Never use backticks with `gh release create --notes` - the shell interprets
  them as command substitution. Always `--notes-file` with a heredoc (write
  the notes scratch into `.hermes/tmp/`, never `/tmp`):

## Heredoc template + attach command

```bash
cat > .hermes/tmp/notes.md << 'EOF'
**[Compare vX.Y.Z...vX.Y.Z](https://github.com/PlayForm/Aphrodite/compare/vX.Y.Z...vX.Y.Z)**

## Aphrodite vX.Y.Z 💋 Plugin vA.B.C

### Summary
One paragraph. What this release is. 2-3 sentences.

### Changes
- **Feature**: description
- **Fix**: description

### Infrastructure
- Build: `cargo build --release -p aphrodite` ✅
- Tests: `cargo test -p aphrodite` ✅ (NNN passed)
- Python: `ruff check` + `pyright` ✅
- Lint: `cargo clippy` ✅

### What Ships
Build.yml's 4-target matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`) stages, per target, the
`aphrodite` binary + `libaphrodite_hermes` dylib + a `SHA256SUMS-<target>.txt`
(4 targets × 3 files = 12 artifacts).

| Artifact pattern | Platform |
|------------------|----------|
| `aphrodite-<target>` / `libaphrodite_hermes-<target>.{so,dylib,dll}` | per matrix target |
| `SHA256SUMS-<target>.txt` | checksums for that target's two binaries |
| Plugin vA.B.C | Hermes (standalone repo) |

### Links
- **Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/...
- **CHANGELOG.md**: [CHANGELOG.md](CHANGELOG.md)
- **Plugin**: https://github.com/PlayForm/Aphrodite-Hermes
EOF
# Normal tag pushes auto-attach every staged artifact; the glob form below is
# only for a from-source re-attach. Glob every staged file, never name 2 of 12:
gh release create Aphrodite/vX.Y.Z --notes-file .hermes/tmp/notes.md \
	staging/aphrodite-* staging/libaphrodite_hermes-* staging/SHA256SUMS-*.txt
```