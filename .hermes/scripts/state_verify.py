#!/usr/bin/env python3
"""Verify the split without a build: every `<Domain>.<Name>` access must name a
field or a method that domain's sub-struct actually declares.

A field rename that missed a site, or a site that picked the wrong domain, shows
up here as an unknown name. This is a name-resolution check, not a type check -
the verification card owns the compiler.
"""

import re
import pathlib

STATE = pathlib.Path("crates/aphrodite/Source/State")
ROOT = pathlib.Path("crates")

DOMAINS = [
    "Store",
    "Marker",
    "Event",
    "File",
    "Turn",
    "ChainSplit",
    "Directive",
    "PollWorker",
    "Configuration",
]

fields = {}
for domain in DOMAINS:
    text = (STATE / domain / "Struct.rs").read_text()
    fields[domain] = set(re.findall(r"^\tpub ([A-Za-z0-9_]+):", text, flags=re.M))

methods = {domain: set() for domain in DOMAINS}
for path in STATE.rglob("*.rs"):
    text = path.read_text()
    for block in re.finditer(r"impl ([A-Za-z0-9_]+) \{", text):
        for name in re.findall(r"\tpub(?:\([a-z]+\))? fn ([A-Za-z0-9_]+)", text):
            for domain in DOMAINS:
                if domain in str(path):
                    methods[domain].add(name)

print("== FIELDS PER DOMAIN ==")
total = 0
for domain in DOMAINS:
    total += len(fields[domain])
    print(f"  {domain:14} {len(fields[domain]):2}  {', '.join(sorted(fields[domain]))}")
print(f"  TOTAL {total}")

print("== METHODS PER DOMAIN ==")
for domain in DOMAINS:
    print(f"  {domain:14} {', '.join(sorted(methods[domain])) or '-'}")

unknown = []
seen = {}
for path in sorted(ROOT.rglob("*.rs")):
    for number, line in enumerate(path.read_text().splitlines(), 1):
        for match in re.finditer(r"\.(" + "|".join(DOMAINS) + r")\.([A-Za-z0-9_]+)", line):
            domain, name = match.group(1), match.group(2)
            known = fields[domain] | methods[domain]
            if name in known:
                seen[(domain, name)] = seen.get((domain, name), 0) + 1
            else:
                unknown.append((str(path), number, domain, name, line.strip()[:120]))

print("== UNKNOWN <Domain>.<Name> ==")
for entry in unknown:
    print(f"  {entry[0]}:{entry[1]}: {entry[2]}.{entry[3]}   {entry[4]}")
print(f"  total unknown: {len(unknown)}")

print("== UNUSED FIELDS (declared, never read through the domain path) ==")
for domain in DOMAINS:
    for field in sorted(fields[domain]):
        if (domain, field) not in seen:
            print(f"  {domain}.{field}")
