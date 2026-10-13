# Task batch checkpoint evidence and unresolved recovery

Date: 2026-10-12. Daji 3.6.x local source follow-up based on `f45f1e1f`.
No version change, commit, push, packaging, installation or remote execution.

## Failure and repair

Preparation previously treated zero errors as enough to authorize execution,
without checking task coverage, source identity or counter agreement. Execution
checkpoint creation checked results, but discarded the supporting task identities;
verification then trusted a saved completion flag and derived policy. Editing the
summary and recomputing its checksum could bypass those original checks. Recovery
also selected only declared failed cases even when other cases had no valid result.

- Preparation now requires matching batch/phase identity, exact integer counters,
  one summary per task, matching task/operator/digest identity, verified program
  identity and aligned error codes/failure receipts. A bare zero-error summary is
  rejected rather than promoted to prepared. Actual preparation failures remain
  inspectable and keep their repair action.
- Checkpoints retain compact preparation evidence and completed-task identities.
  Full task inputs, solver outputs and successful admission reports are not copied.
  Execution evidence is derived from checked rows, never adopted from a caller's
  supplied completion-evidence field. Its batch-binding marker is recomputed when
  read, as are completion and the ordered incomplete-case list.
- Completion requires integer counters, aligned readiness totals, the original
  batch/phase, executed status and complete unique task coverage. Contradictory
  execution input remains diagnostic/noncomplete; it cannot authorize archival.
  A rehashed saved flag, altered task identity or inconsistent unresolved list
  fails checkpoint verification and resume planning.
- Missing results remain unresolved alongside known failures. A foreign batch or
  wrong phase, including an error-only summary, selects inspection/resolution
  rather than direct retry. No automatic task execution or replay is introduced.
- Public routes return HTTP 422 with `operator_task_checkpoint_evidence_invalid`
  for failed consistency checks. Errors name the failed field without dumping
  result payloads. Existing structural errors retain their own codes.

One older partial-execution test used only manually written counters and no actual
successful-task record. It now runs a real two-task batch, with one deliberately
invalid digest. Its original targeted-retry assertions remain; a new separate
test requires all unresolved targets when the successful peer's record is absent.

## Verification

The initial 11-test selection produced nine failures before the repair. These are
related boundary assertions, not nine independent security vulnerabilities.
The final addition contains 15 direct tests and one public API test.

- Preparation mutations cover count type/value, batch/phase, missing/duplicate
  rows, row identity/status, error-code counts and failure receipts. Successful
  preparation survives JSON exchange; actual failed preparations remain blocked
  and reject inconsistent mutations.
- Persisted evidence mutations include malformed containers, missing evidence,
  duplicate/foreign task identities, changed digests, completion flags, incomplete
  targets and batch binding. Checksums are recomputed with the existing encoding,
  so rejection is not merely a stale-hash check.
- All seven completion counters have missing, floating and negative controls;
  readiness totals also reject floating count tokens. Empty batches still prepare,
  execute and verify successfully.
- Known failures plus missing records keep both cases in the resolution plan.
  Foreign mixed and error-only summaries cannot authorize direct retry; matching
  corrected evidence retains ordinary targeted retry.
- Adding 700,000 bytes to a successful result produces an identical checkpoint
  with the same fixed creation time. This verifies result-payload independence,
  not constant storage with respect to task count or a measured scale benchmark.
- Public HTTP tests reject a bare preparation summary, accept actual preparation
  and execution, reject a rehashed false completion through verification and resume
  routes, and continue to accept the unchanged complete checkpoint.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68114
```

Final service result: 2,599 total, 2,528 passed, 71 conditional skips, zero
failures. The expanded suite also passed before the final strict readiness-count
check; the command above is the final rerun. Conditional skips are not executed
evidence. Mix required its local TCP synchronization socket; no remote host or
live Agent was used.

Formatting, whitespace, document-book/inventory, tensor self-test/validation and
project organization checks pass. Files remain within the 800-line source and
2,000-line document limits, with zero tracked size debt. The tensor still has
four maturity, 20 evidence-grade and 15 P0 gaps; release qualification stays blocked.

## Compatibility and limits

Legacy preparation/execution summaries without compact task evidence no longer
authorize reuse. Regenerate explicitly from retained original preparation/results;
do not fabricate evidence from a boolean or silently rerun jobs. Evidence-free
draft checkpoints remain supported. There is no migration that certifies records
whose supporting evidence has been lost.

This is control-plane consistency, not authenticated execution provenance. A party
able to rewrite every record and checksum consistently is not authenticated by
this mechanism. There is no signature, independent execution ledger, physical
validation, new native Engine test coverage or installed/remote qualification.
Legacy JSON digest normalization is unchanged. The checkpoint stores identities,
not complete successful Agent receipts or numerical outputs.

Execution failure-receipt/counter partition coherence, cross-process attempt
ownership, producer readiness retention and durable restart need further scoped
work. Diagnostic execution summaries may remain contradictory while explicitly
blocked; this repair does not claim that every stored failure summary is true.
Numerical algorithms and Engine/Agent execution contracts are unchanged.
