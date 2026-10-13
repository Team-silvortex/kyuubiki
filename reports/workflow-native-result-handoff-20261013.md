# Bounded native workflow result handoff

Date: 2026-10-13. Scope: managed native diagnostic/scoring workflow recovery.

## Gap and implementation

The original runtime returned native success without committing its receipt to
the parent recovery record. A later generation could only recover from the
original Agent's short-lived memory cache. The first regression (seed 68149)
failed because the handoff callback was never called before returning the output.

Managed native execution now commits a verified success before downstream
publication, using the existing leased, generation-fenced job/result transaction.
Receipts recovered from the original Agent also take this handoff path. Reuse of
an existing checkpoint is read-only and does not refresh or rewrite it.

The private `native_result_checkpoints` bucket binds each successful receipt to:

- Exact attempt, TaskIR digest, operator and program identity.
- SHA-256 of the entire selected dispatch observation, including its endpoint,
  request ID, parent, state and timestamps, without another raw observation copy.
- Parent execution envelope and the producer claim, which cannot precede the
  original dispatch claim or exceed the current claim.
- Lossless receipt JSON and its digest, preserving integral floats and tiny values
  through SQL/JSON storage.

Public recovery exposes `native_result_checkpoint_policy` and
`native_result_checkpoint_count`, not receipt payloads. This is an internal
managed-workflow facility, not a new public receipt endpoint or SDK language API.
Unmanaged calls retain their existing behavior unless a trusted internal callback
is supplied; request graph JSON cannot supply such callbacks.

## Bounds and failure behavior

Per parent: 32 entries, 512 KiB per receipt, 65,536 decoded JSON nodes per receipt,
and 2 MiB encoded bucket bytes. This is an explicit additional bound, separate
from solve artifacts and branch decisions, not an unlimited snapshot archive.
Byte budgeting stops before creating an oversized full JSON copy.

Full or oversized handoffs omit new entries without evicting completed evidence.
Missing handoff falls back only to the original Agent receipt. If neither is
available, recovery blocks, with no automatic execution, alternate-attempt lookup
or silent output publication. Store errors are not cache misses. Unknown or
competing dispatches, input changes, missing journals and replay restrictions
remain blocking even when a checkpoint exists.

Storage rejection, invalid receipt, lost ownership or changed observation cannot
release success downstream. The journal and claim are checked again after a
potentially slow handoff. Checkpoint faults use explicit native recovery reasons
and a checkpoint-specific next action; `on_error: skip` cannot suppress them.
Ordinary completion/failure/cancellation prunes the bucket. Recovery-blocked
parents retain it, and even malformed residual native checkpoint keys prevent
premature dispatch-history release in compact SQLite/PostgreSQL/memory projections.

## Verification

The new tests cover runtime handoff, private transaction/readback, corruption,
ownership, ambiguity, caps and application restart. Ten restart cases use the
actual application/coordinator/store and controlled TCP producers for both native
extract and transform routes. After the first native success reaches a held
downstream solve, the entire application is stopped and all Agent endpoints are
removed. Valid handoff resumes only the unfinished solve after restart. Missing
checkpoint, corrupt checkpoint, lost journal and restricted replay instead retain
blocked recovery. No producer fetch or re-execution is possible in the successful
offline cases. Those are controlled transport fixtures, not real Rust solvers.

Added **51 regressions**, plus extra retention and skip-on-error assertions in
existing suites. Final expanded SQLite run (seed 68157): **3,006 total, 2,935
passed, 71 conditional skips, zero failures**. Final focused durable-memory run
(seed 68156): **81 total, 80 passed, one SQL-only pool test skipped, zero failures**.
Both final runs deny warnings. The earlier focused SQLite run (seed 68153) passed
66 tests before the final four runtime boundaries and retention assertion were
added; the final expanded run includes those additions.

Formatting, whitespace, tensor validation/self-test, HTML book, documentation
inventory and project-organization checks passed. Source/document limits remain
800/2,000 lines with no tracked debt. Tensor structure still has zero structural
gaps, four maturity gaps, 20 evidence-grade gaps and 15 P0 items; its release gate
remains blocked. This adds bounded evidence without promoting those open gaps.

Reproduction from `apps/web`:

```text
mix test test/kyuubiki_web/orchestra/workflow_native_result_handoff_test.exs \
  test/kyuubiki_web/orchestra/workflow_native_result_checkpoint_test.exs \
  test/kyuubiki_web/orchestra/workflow_native_result_restart_test.exs \
  test/kyuubiki_web/orchestra/workflow_native_recovery_block_test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch_retention_test.exs \
  --seed 68156 --warnings-as-errors

mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api \
  test/kyuubiki_web/orchestra/workflow_*test.exs \
  test/kyuubiki_web/orchestra/operator_task*test.exs \
  test/kyuubiki_web/orchestra/operator_dispatch*test.exs \
  test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs \
  test/kyuubiki_web/playground test/kyuubiki_web/library \
  test/kyuubiki_web/storage --seed 68157 --warnings-as-errors
```

The memory run additionally sets `KYUUBIKI_STORAGE_BACKEND=memory` and an isolated
temporary `KYUUBIKI_DATA_DIR`. That owned temporary directory was removed after testing;
no installed service, user study or unrelated build cache is modified.

## Limits

No new real Rust Agent, remote Linux, live PostgreSQL, installed desktop, physical
solver accuracy, scale or power-loss qualification is claimed. The SQL projection
has a PostgreSQL expression, but this round does not execute a live PostgreSQL test.
The existing Agent cache TTL, entry count and byte budget remain unchanged.

This does not close the gap between an Agent completing work and the control plane
receiving, observing and committing its receipt. A lost reply or crash before that
handoff remains governed by the original unknown/receipt-unavailable fences.
Results beyond retention limits still depend on the original Agent. Joint rollback
of both stores, malicious provenance replacement, cross-process Agent result
ownership and an atomic producer-to-database protocol remain open. No automatic
replay or exactly-once guarantee is added, and tensor release gaps are not promoted.
