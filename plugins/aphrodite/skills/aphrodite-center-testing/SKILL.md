---
name: aphrodite-center-testing
description: "Test aphrodite center features end-to-end — call site audit, orphaned function detection, preview depth verification across all compression paths."
version: 1.0.0
---

# Aphrodite Center Testing

Test the CCR center feature (v0.5.84+) across all compression paths.

## Quick Test
Run `aphrodite_test mode=full` to verify basic compression/retrieval, then:
```python
# Test center in structure line
curl -s -X POST http://127.0.0.1:9798/tool/relay \
  -H "Content-Type: application/json" \
  -d '{"tool":"aphrodite_compress","params":{"content":"fn foo() {}","type":"code","_ccr_center":"code_rust"}}'
```
Expected: `[compress: ln=1;center=code_rust]` in structure line.

## References
- `references/center-feature-audit.md` — Full v0.5.86–v0.5.90 audit
