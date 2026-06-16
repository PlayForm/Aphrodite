# CCR Catalog Display Fixes

## Bug 1: `CCR:{}` Empty Hash Entries

**Symptom**: Catalog shows entries like `CCR:{} | 0B | <preview>`

**Root cause**: `_parse_ccr_markers` stores `parts[0]` as hash without validation. When a marker like `<<<CCR:|compress|0>>>` is parsed, `parts[0]` is empty string or dict.

**Fix layer 1 — Parse guard** (line ~595):
```python
'hash': str(parts[0]) if parts[0] else '',
```
Cast to str(), handle falsy values.

**Fix layer 2 — Parse filter** (line ~606):
```python
return [m for m in markers if m['hash'] and len(m['hash']) >= 4]
```
Skip markers with empty/short hashes.

**Fix layer 3 — Display guard** (line ~738):
```python
h = str(m.get('hash', '')).strip()
if len(h) < 4 or h in ('{}', '?', 'None', 'null', 'undefined'):
    continue
```
Three-layer defense: cast → filter → skip.

## Bug 2: `|tool|56148|token>>>` in Previews

**Symptom**: Catalog previews show marker fragments like `|tool|56148|token>>> {"success": true...`

**Root cause**: `_extract_preview` at line ~819 splits on `]` (old bracket format `[CCR:...]`) but current format is `<<<CCR:...>>>`.

**Original broken code**:
```python
if after.startswith('|'):
    after = after.split(']', 1)[-1] if ']' in after else after
```

**Fix**:
```python
if '>>>' in after:
    after = after.split('>>>', 1)[-1].strip()
```

## Bug 3: Preview Text Starting Inside Marker

**Symptom**: Previews include `|token>>>` fragments even after fix #2

**Root cause**: `_parse_ccr_markers` used `re.findall()` which returns captured groups only, no position info. The text position was computed via `text.find(captured_group)` which finds the wrong occurrence.

**Fix**: Use `re.finditer()` for match objects with `match.end()`:
```python
for match in _CCR_RE.finditer(text):
    m = match.group(1)
    marker_end = match.end()  # position right after >>>
    preview = text[marker_end:].strip()[:200]
```

## Pattern

All three bugs follow the same pattern: **validate at parse → filter at display**. Never trust raw data from regex captures. Always cast, validate length, and guard before displaying.
