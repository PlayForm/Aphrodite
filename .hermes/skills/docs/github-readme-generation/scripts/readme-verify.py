#!/usr/bin/env python3
"""Mechanical verification for a restyled README.

Usage: python3 readme-verify.py README.md [--no-net]
Checks (exit 1 on any failure):
  1. Every heading containing an emoji has U+2001 (em-quad) in it.
  2. Every GFM alert ([!NOTE]/[!WARNING]/[!IMPORTANT]/[!TIP]) has the tag
     alone on a '>' line, a blank '>' line, then '> body'.
  3. Every img.shields.io badge URL returns HTTP 200 (skipped with --no-net).
  4. Bottom link-reference definitions ([name]: <target>) whose target is a
     relative path resolve to an existing file.
  5. No bare '-' / '+' lines inside ```diff fences (detects write-tool
     trailing-space stripping of verbatim ' - ' / ' + ' lines).
"""

import os, re, sys, urllib.request


def fail(msg):
    print("FAIL:", msg)
    sys.exit(1)


path = sys.argv[1] if len(sys.argv) > 1 else "README.md"
no_net = "--no-net" in sys.argv
c = open(path, encoding="utf-8").read()
lines = c.splitlines()

emoji = re.compile(r"[\U0001F300-\U0001FAFF\u2600-\u27BF]")
for i, line in enumerate(lines, 1):
    if line.lstrip().startswith("#") and emoji.search(line) and "\u2001" not in line:
        fail(f"L{i}: emoji heading without em-quad: {line[:60]}")

alerts = re.findall(r"> \[!(NOTE|WARNING|IMPORTANT|TIP)\][^\n]*\n([^\n]*)\n([^\n]*)", c)
for tag, l2, l3 in alerts:
    if l2.strip() != ">" or not l3.startswith("> "):
        fail(f"GFM alert [!{tag}] malformed (tag alone on '>', blank '>' line, then '> body')")

if not no_net:
    for url in re.findall(r"https://img\.shields\.io/static/v1[^)\s\"]+", c):
        try:
            code = urllib.request.urlopen(url, timeout=10).getcode()
            if code != 200:
                fail(f"badge HTTP {code}: {url}")
        except Exception as e:
            fail(f"badge unreachable: {url} ({e})")

refs = re.findall(r"^\[([^\]]+)\]:\s*([^\s]+)\s*$", c, re.M)
for name, target in refs:
    if not target.startswith(("http://", "https://", "mailto:")) and not os.path.exists(target):
        fail(f"link-ref [{name}] target missing: {target}")

in_diff = False
for i, line in enumerate(lines, 1):
    if line.startswith("```diff"):
        in_diff = True
        continue
    if line.startswith("```"):
        in_diff = False
        continue
    if in_diff and line in ("-", "+"):
        fail(f"L{i}: bare '{line}' inside diff fence (trailing space stripped?)")

print(f"OK: {path} - {len(alerts)} alerts, {len(refs)} link-refs")
