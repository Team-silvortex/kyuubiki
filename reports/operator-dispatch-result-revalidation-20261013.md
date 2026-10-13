# Public receipt-read observation revalidation

Date: 2026-10-13. Scope: shared Orchestra receipt API and native workflow recovery.

## Reproduced gap

Agent receipt expiry, eviction and process restart already produced unknown results
without automatic replay. However, the public `fetch-dispatch-result` path checked
its journal observation only before querying the Agent. The workflow-native caller
performed a later check, but the shared API could return a recovered completion
after its original local observation disappeared during the lookup.

The new disappearance test failed against the original implementation with seed
68144: deleting the selected journal file inside the controlled probe still returned
`receipt_recovered`, `executed` and the result payload. The repaired test rejects
that stale completion.

## Repair

`OperatorDispatchJournal.verify_observation/2` compares the entire selected record
with the current cached observation and verifies current inventory plus the selected
on-disk record. The public result path calls it after lookup, including unknown and
unreachable Agent replies. No-result queries that selected no observation do not
send an RPC and retain the existing unknown response.

- Missing/corrupt/replaced/partial storage or process loss cannot return a stale
  result. Actual storage damage retains the existing quarantine behavior.
- Legitimate concurrent state transitions or ordinary retention eviction return
  `operator_task_dispatch_observation_changed`, rejecting only this read. Healthy
  journal operations and a later explicit read remain available.
- Restarting the journal with the exact same durable observation permits the read;
  restarting it empty does not. No journal rewrite, retention renewal, extra archive
  or automatic retry is introduced.
- Only the selected payload is revalidated. Unrelated payload corruption remains
  subject to the established selection/startup checks, not a new full scan.
- Native recovery maps these errors to its existing observation-changed and
  journal-unavailable blocking reasons. Its additional parent-wide ambiguity,
  durable evidence and execution-claim checks remain in place.

The HTTP boundary returns 422 with an error code and no completion when validation
fails. Controlled TCP tests check that error responses expose neither injected raw
storage content nor local paths, and that no job publication or second Agent request
occurs. A simulated new Agent incarnation returning no retained result stays unknown.

The successful v1 envelope and SDK-facing schema shape are unchanged. The schema
description and HTML/SDK guidance now state the post-lookup verification requirement.

## Verification

Added **28 regressions**: 20 focused observation/result tests and eight public API
tests using a controlled local TCP Agent. These cover deletion, corruption, valid
replacement, partial writes, missing root, unavailable journal process, retention
eviction, legal transitions, unchanged restart, lost restart, malformed verification
requests, narrow payload selection and unavailable receipts.

Final focused SQLite suite (seed 68146): **130 passed, zero failures**, warnings denied.
Final focused durable-memory suite (seed 68148): **130 passed, zero failures**, warnings denied.
Expanded SQLite suite (seed 68147): **2,955 total, 2,884 passed, 71 conditional skips,
zero failures**, warnings denied.

Formatting, whitespace, coverage-tensor validation and self-test, HTML book,
documentation inventory and project-organization audits passed. Source/document
limits remain 800/2,000 lines. Tensor gaps remain four maturity gaps and 20 evidence-
grade gaps with 15 P0 items; this bounded repair does not promote release readiness.

```text
# Run from apps/web.
mix test test/kyuubiki_web/api/operator_dispatch_storage_api_test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch_result_test.exs \
  test/kyuubiki_web/orchestra/workflow_native_attempt_recovery_test.exs \
  test/kyuubiki_web/orchestra/workflow_native_result_recovery_test.exs \
  --seed 68146 --warnings-as-errors

mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api \
  test/kyuubiki_web/orchestra/workflow_*test.exs \
  test/kyuubiki_web/orchestra/operator_task*test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch*test.exs \
  test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs \
  test/kyuubiki_web/playground test/kyuubiki_web/library \
  test/kyuubiki_web/storage --seed 68147 --warnings-as-errors
```

The memory run sets `KYUUBIKI_STORAGE_BACKEND=memory` and uses an isolated temporary
`KYUUBIKI_DATA_DIR`. That owned directory is removed after validation; no unrelated
build caches, user research data or installed services are changed.

## Limits and next work

This is a point-in-time integrity check, not an atomic transaction spanning the
journal, Agent cache and subsequent user actions. Unavailability remains distinct
from successful recovery. It does not lengthen the Agent's 600-second reservation-
origin TTL, 64-entry/32-MiB cache, or its process-only lifetime.

No new real Rust solver, physical-reference, remote server, live PostgreSQL,
installed-App, power-loss or multi-writer qualification is claimed. Existing native
cache tests and earlier live restart evidence were inspected, not rerun this round.
There is no additional SDK language implementation or new replay authority.

Producer receipt availability for long-running workflows, durable result handoff
and cross-process ownership remain separate work. Joint rollback/loss of both
stores and malicious provenance forgery are not solved by this check.
