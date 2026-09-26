# Auto-expand fixture acceptance test (C-001)

Probe procedure consumed by the C-001 section of `aphrodite-development`
(SKILL.md). Run it whenever an auto-expand claim must be decided: the test's
outcome is either "configuration is inert" or "a real consumer is active".

An auto-expand field is configuration observability only until an active
consumer is shown in the running binary. The fixture test is the explicit
before/after test that shows it. The compression threshold referenced in step
1 is the session's CCR compression threshold.

Procedure:

1. Create a known payload larger than the compression threshold.
2. Read it once under the proposed auto-expand setting.
3. Record whether the response is inline content, a valid marker, or a
   malformed result.
4. If it is a marker, resolve it once through the canonical retrieval tool.
5. Compare bytes or normalized text with the source payload.
6. Record the runtime binary version and configuration source.

Expected observation with the current source: a marker appears and requires
canonical retrieval, which shows the configuration is inert. If the test ever
shows resolution without retrieval, a real consumer is active: run the
reactivation gate in `aphrodite-auto-expand-testing` and update the record
from `inactive` to `active`. The record update is owned by that skill, not by
this one.
