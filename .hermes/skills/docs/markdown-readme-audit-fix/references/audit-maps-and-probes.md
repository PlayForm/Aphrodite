# Audit maps and probes

Reference data for `markdown-readme-audit-fix`: the minimal and full dash
character maps, the structure-tree probe, the backticking term list, and the
corruption-scan patterns.

## Minimal character map (6-char set)

Used with `--check` mode of `scripts/dash-normalizer.py` and for every manual
dash/quote edit:

| Unicode    | Name                                     | Replace with                                                                                                                                                         |
| ---------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| U+2014 `-` | EM DASH                                  | Context-dependent: `-` → `.  `, `,  `, or `:  ` as prose punctuation; when both sides are equally weighted, use `-` (en dash also acceptable; plain `-` is simplest) |
| U+2013 `-` | EN DASH                                  | `-`                                                                                                                                                                  |
| U+2018 `‘` | LEFT SINGLE QUOTATION MARK               | `'`                                                                                                                                                                  |
| U+2019 `’` | RIGHT SINGLE QUOTATION MARK / APOSTROPHE | `'`                                                                                                                                                                  |
| U+201C `“` | LEFT DOUBLE QUOTATION MARK               | `"`                                                                                                                                                                  |
| U+201D `”` | RIGHT DOUBLE QUOTATION MARK              | `"`                                                                                                                                                                  |

## Full regex set (24 characters)

The former normalize-dashes hook applied this broader set to every file written
by the agent. The live hook covers only the common dash codepoints (U+2010-U+2015
range); use this full set as the scan set for documentation audits:

```
U+058A  Armenian hyphen         U+2E17  Double oblique hyphen
U+05BE  Hebrew maqaf             U+2E1A  Hyphenation point
U+1400  Canadian syllabics hyp   U+2E3A  Two-em dash
U+1806  Mongolian soft hyp       U+2E3B  Three-em dash
U+2010  Hyphen                   U+2E40  Double hyphen
U+2011  Non-breaking hyphen      U+2E5D  Oblique hyphen
U+2012  Figure dash              U+301C  Japanese wave dash
U+2013  En dash                  U+3030  Double vertical line
U+2014  Em dash                  U+30A0  Kana double hyp
U+2015  Horizontal bar           U+FE31  Vert. em dash (pres)
U+FE32  Vert. en dash (pres)     U+FE58  Small em dash
U+FE63  Small hyphen-minus       U+FF0D  Fullwidth hyphen-minus
```

Scan command: `python3 scripts/dash-normalizer.py --full`.

## Structure-tree probe

```python
import os
src = "crates/aphrodite/src/"
for root, dirs, files in os.walk(src):
    dirs.sort()  # deterministic order
```

Build the ASCII tree from the `ls` output, not from memory.

## Bulk backticking probe

Order matters: longer/more-specific terms FIRST, short terms LAST (short terms
won't corrupt substrings):

```python
TECH_TERMS = [
    "aphrodite", "Aphrodite", "BLAKE3", "Hermes", "ctypes", "dylib",
    "SQLite", "Tokio", "WebAssembly",
    "Rust", "JSON", "clippy", "ruff",  # short terms LAST (won't corrupt substrings)
]

for line in prose_lines:
    for term in TECH_TERMS:
        if f'`{term}`' in line:
            continue  # already formatted
        line = re.sub(r'(?<!\w)' + re.escape(term) + r'(?!\w)', f'`{term}`', line)
```

CRITICAL - post-pass substring corruption check. The auto-formatter corrupts
words containing short terms: `` `Wind`ows `` → fix back to `Windows`;
`` `Rest`art `` → `Restart`. Always run the corruption sweep after backticking.

## Post-pass corruption scan patterns

```python
import re

patterns = {
    # Backtick inside HTML attribute URLs
    r'(src|href)="[^"]*`[^"]*"': "HTML attribute URL backtick",
    # Backtick inside markdown link URLs
    r'\]\(https?://[^)]*`[^)]*\)': "Markdown link URL backtick",
    # NGI0 Commons Fund corruption (Common → `Common`)
    r'NGI0 `Common`s Fund': "NGI0 Commons corruption",
    # Windows corruption (Wind → `Wind`)
    r'`Wind`ows': "Windows corruption",
    # Mid-URL backtick style
    r'https://[a-zA-Z.]+`[a-zA-Z]': "Mid-URL backtick",
}

for filepath in all_touched_files:
    with open(filepath) as f:
        content = f.read()
    for pattern, label in patterns.items():
        matches = list(re.finditer(pattern, content))
        if matches:
            print(f"\n--- {label} in {filepath} ---")
        for m in matches:
            line_no = content[:m.start()].count('\n') + 1
            print(f"  L{line_no}: {m.group()[:100]}")
```

Do NOT flag: ``[`url`][ref]`` - intentional markdown link text formatting.

Real corruptions to fix:

| Pattern                                                   | Fix                          |
| --------------------------------------------------------- | ---------------------------- |
| `src="https://...` backtick                               | remove backtick, restore URL |
| `NGI0 `Common`s Fund`                                     | `NGI0 Commons Fund`          |
| `` `Wind`ows ``                                           | `Windows`                    |
| Any backtick between `:` and domain continuation in a URL | remove the rogue backtick    |
