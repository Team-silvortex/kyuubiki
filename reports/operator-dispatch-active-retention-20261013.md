# Active workflow dispatch-history retention

Date: 2026-10-13. Scope: local Orchestra recovery and storage retention.

## Reproduced gap

The journal previously kept the latest 128 terminal observations without checking
whether their actual parent job still needed them. A live workflow's older native
attempt could disappear when unrelated tasks turned over that window. The durable
attempt bucket correctly blocked recovery after this loss, but routine cleanup
had needlessly made a recoverable workflow unavailable.

A new test first failed against the old implementation (seed 68140): after 140
unrelated terminal observations, the active parent's attempt was absent. The same
test now passes both before and after restarting the journal.

## Retention contract

- Protect observations bound to an active parent, including missing result state.
- Protect recovery-blocked parents and inconsistent retention metadata: unknown
  schemas/states, null or array recovery values, mismatched terminal states and
  retained envelope/evidence keys, including JSON null values.
- A surviving recovery record without a job remains protected. SQL foreign keys
  normally prevent this orphan; the memory backend test exercises it directly.
- Matching ordinary completed/failed/cancelled jobs release their history after
  the private recovery envelope and native attempt bucket are pruned.
- Healthy absence of both job and recovery is ordinary standalone/deleted history.
  Explicit joint deletion leaves no separate pin registry or additional archive.
- Unbound/legacy observations cannot infer a parent from task names. Their terminal
  history remains ordinary history; unresolved observations are never pruned.
- The ordinary terminal window remains 128. Protected records do not consume this
  window, but all records count toward the unchanged 512-record hard cap and
  4,096-byte per-record limit. Saturation rejects new work, not required evidence.

The check uses compact job status and recovery-retirement metadata, not a full
integrity verification of unrelated result payloads. It does not make retained
observations into execution receipts or grant replay authority.

## Concurrency and failure isolation

The dispatch caller obtains bounded terminal candidates from the journal and
queries at most 512 distinct parents outside the journal process. SQL selects
only status and small recovery projections within a transaction; durable memory
projects the shared jobs/results state in one read. No scientific output, graph
or model payload is copied into the query result.

Only candidate records still exactly equal to the observed generation may be
pruned. New observations cannot inherit an old caller's cleanup decision.
Concurrent admissions can temporarily retain more ordinary history until the
next reservation reconciles it; the hard cap still applies.

Keeping storage waits outside the journal avoids a journal/lease-owner wait
cycle. A single-connection SQL pool test holds the connection, starts an admission
waiting for retention metadata, and confirms that journal reads still complete.
After the connection is released, admission completes normally.

Unavailable storage rejects admission without deleting records or latching a
journal corruption error. Healthy reads continue and the next admission succeeds
after storage returns. Actual journal corruption still follows the existing
fail-closed quarantine policy.

## Verification

Added 12 tests: 11 retention cases and one managed-native recovery integration.
The blocked/malformed metadata case covers multiple variants for each backend.

Targeted SQLite run (seed 68141): **58 passed, zero failures**.
Targeted durable-memory run (seed 68142): **57 passed, one SQL-only skip**.

Expanded SQLite service regression (seed 68143): **2,927 total, 2,856 passed,
71 conditional skips, zero failures**. Areas include workflow runtime/coordinator,
native evidence, operator dispatch/batch recovery, API admission, playground,
library and storage.

Formatting, diff whitespace, HTML book (26 files), documentation inventory,
tensor self-test/validation and project organization audit all pass. Source/doc
limits remain 800/2,000 lines with zero tracked debt. Tensor maturity is not
promoted: 13 modules, 11 paradigms, zero structural gaps, four maturity gaps,
20 evidence-grade gaps and 15 P0 gaps; release status remains blocked.

```text
# Run from apps/web.
mix test test/kyuubiki_web/orchestra/operator_dispatch_retention_test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch_journal_test.exs \
  test/kyuubiki_web/orchestra/workflow_native_dispatch_evidence_test.exs --seed 68141

mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api \
  test/kyuubiki_web/orchestra/workflow_*test.exs \
  test/kyuubiki_web/orchestra/operator_task*test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch*test.exs \
  test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs \
  test/kyuubiki_web/playground test/kyuubiki_web/library \
  test/kyuubiki_web/storage --seed 68143
```

For the memory run, use `KYUUBIKI_STORAGE_BACKEND=memory` and an isolated temporary
`KYUUBIKI_DATA_DIR`. Only this round's explicit temporary memory directory is
removed afterwards; no user data or unrelated test evidence is swept.

## Remaining boundaries

No live PostgreSQL, remote server, installed application, Rust solver, power-loss
or multi-writer acceptance is claimed. The PostgreSQL projection uses the same
adapter-supported JSON extraction contract but was not exercised on a live server.
No new Engine or physical-computation qualification is claimed.

This does not protect against coordinated rollback/loss of both stores, hostile
database mutation, arbitrary reuse of a retired parent ID, or missing legacy
native evidence. Normal admissions use unique parent identities and terminal
jobs do not reopen through the supported workflow API.

Journal retention does not extend an Agent's bounded receipt lifetime. A retained
attempt whose original receipt has expired or whose Agent restarted still requires
explicit fail-closed handling. Producer-side receipt/readiness lifetime and
cross-process ownership remain follow-up work; no exactly-once claim is added.
