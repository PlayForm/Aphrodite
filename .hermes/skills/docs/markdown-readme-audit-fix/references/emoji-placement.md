# Emoji placement matrices

Worked matrices for §5 of `markdown-readme-audit-fix`. All four types must
read back as zero via `scripts/emoji-scanner.py` before the pass is complete.

## Type matrix

| Type   | Pattern                                                                   | Fix                                                                     |
| ------ | ------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| TYPE A | Regular ASCII space before emoji in `# heading`                           | Replace ` ` with file-native em-quad (`\u2001` or `&#x2001;`)           |
| TYPE B | Double em-quad (two in a row: `&#x2001;&#x2001;`)                         | Collapse to single `&#x2001;`                                           |
| TYPE C | Em-quad between base emoji and Fitzpatrick modifier (e.g. `🙏&#x2001;🏿`) | Remove the quad: `🙏🏿`                                                   |
| TYPE D | Emoji appears on the LEFT of prose text                                   | Move emoji to the RIGHT end of the text segment, preceded by an em-quad |

## TYPE D - emoji-left-of-text (most widespread)

Universal rule: every emoji is preceded by an em-quad and appears **at the
end** of a heading, sentence, or labeled item - never before the prose it
decorates.

| Sub-pattern         | Before (wrong)                                 | After (correct)                                |
| ------------------- | ---------------------------------------------- | ---------------------------------------------- |
| Link/markup line    | `&#x2001;📖 **[CCR Protocol Docs](...)**`      | `**[CCR Protocol Docs](...)**&#x2001;📖`       |
| Bold-intro sentence | `Welcome to **Aphrodite**💋, the lightweight…` | `Welcome to **Aphrodite**, the lightweight…💋` |
| Heading title       | `## \`CCR\`💋 in \`Aphrodite\`🌐 Ecosystem`    | `## \`CCR\` in \`Aphrodite\`🌐 Ecosystem💋`    |

Manual TYPE-D fix - always the same two-step rewrite:

1. Pick up the emoji and any preceding em-quad from wherever it sits before
   the prose.
2. Place it at the right end of the text, preceded by a single em-quad.

```
Wrong:  <text> <emoji> <more_text>
Right: <text> <more_text> <em-quad> <emoji>
```

## NOT errors - leave as-is

- Table cell status rows: `| Status | 📝 Specified …` (status icon inside cell).
- Ecosystem heading combos: `&#x2001;🍃 + &#x2001;🌐` - the _second_ emoji is
  furthest-right; reorganize so all decorative emoji are after the text.
- Lines where the emoji is the entire content: `<h3>&#x2001;💋</h3>` or
  standalone `&#x2001;🌐`.
- Some files use the actual U+2001 character on _both_ sides of the emoji -
  deliberate; confirm with `od -c` before changing anything.

## Mermaid subgraph/node labels

In mermaid subgraph titles and node labels, emoji must still appear on the
**right**, preceded by an em-quad. The only difference from prose is the label
is quoted.

| Sub-pattern    | Before (wrong)                 | After (correct)               |
| -------------- | ------------------------------ | ----------------------------- |
| Subgraph title | `"&#x2001;⛰️ Proxy - Backend"` | `"Proxy - Backend&#x2001;⛰️"` |
| Node label     | `"&#x2001;🚀 Bootstrap/…"`     | `"Bootstrap/&#x2001;🚀"`      |

The em-quad is still meaningful inside mermaid quotes - it separates label text
from the emoji decoration. Do not omit it. The only forbidden placement is
emoji on the LEFT of the text it decorates.
