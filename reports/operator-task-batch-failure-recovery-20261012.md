# Task batch failure recovery and retry permissions

Date: 2026-10-12. Daji 3.6.x local source follow-up based on `f45f1e1f`,
including the preceding uncommitted checkpoint-evidence repair. No version change,
commit, push, packaging, installation or remote execution.

## Failure and repair

Batch recovery previously selected failed cases without establishing that their
failure receipts belonged to those tasks or agreed with the outcome counters.
Even an explicitly nonretryable failure could produce `retry_failed_cases`.
This follow-up separates receipt consistency, explicit retry advice and repair.

- Failed rows must match their case, task, digest and operator identities. Agent
  failures reuse the existing failure/readiness validator and check the inner
  result receipt against the row receipt. Top-level failure receipts are matched
  by task identity, not array position. Duplicate, missing or mismatched receipts
  cannot supply verified recovery advice.
- Completed, failed, blocked and skipped cases form disjoint, exhaustive batch
  partitions. Exact integer counters, result count, error-code frequencies,
  readiness frequencies and aggregate status must agree. A blocked row requires
  task identity, nondispatchable readiness and a nonblank required action.
- New compact `recovery_evidence` records task identities, critical failure-receipt
  fingerprints and blocked readiness. Creation derives it from checked rows,
  rather than adopting caller-supplied evidence. Readback rechecks the record,
  receipt permissions and counters, including after a caller rehashes a checkpoint.
- `retryable_case_ids` contains verified failures permitting an explicit retry;
  `repair_required_case_ids` contains verified nonretryable failures. Existing
  control-plane failures are nonretryable and require repair before a fresh
  submission. Unsafe-continuation receipts force resolution rather than direct
  retry. `automatic_replay_allowed` is always false. A retry candidate is not a
  current-ownership or side-effect replay authorization.
- Contradictory executions remain readable diagnostics with unverified recovery,
  a static failed-check field and no recovery actions. They cannot borrow stale
  preparation advice. Individually proven successful peers are preserved while
  unresolved cases remain targets. Blank readiness actions are filtered from the
  compact action list so a generated diagnostic checkpoint remains readable;
  the malformed blocked row is still unverified, not repaired or promoted.

Full numerical results are not copied. Fingerprints cover the failure contract's
identity, reason/message/stage and three recovery fields, not arbitrary admission
reports or extra result fields. Storage remains proportional to task/failure count;
this is not a constant-memory or measured scale-performance claim.

## Verification

The initial ten-test selection failed before implementation. These are related
boundary assertions, not ten independent vulnerabilities. The final addition is
twelve direct tests and one public API test, with multiple mutations per test.
Old assertions that suggested direct retry for nonretryable failures now require
repair. Two fabricated readiness tests explicitly check conservative handling of
contradictory data; existing real producer-path blocker controls still pass.

- Receipt mutations cover identity, reason, owner/schema/stage, absent/duplicate
  receipts, permission type/value and required action. Failure rows and Agent
  inner receipts must agree with their outer records. Caller proof is replaced,
  not trusted.
- Counter/status mutations and overlapping or missing partitions cannot produce
  verified recovery. Successful and empty batches survive JSON exchange and keep
  archival behavior. Strict-stop batches preserve skipped targets.
- Rehashed persisted checkpoints reject altered permissions, action, identity,
  counters, malformed proof containers and contradictory verification/error flags.
  Diagnostic checkpoints round trip, retain unresolved targets and do not leak
  rejected values into validation error messages.
- The controlled loopback Agent protocol exercises all four combinations of
  retryable/nonretryable and safe/unsafe continuation through actual service batch,
  checkpoint and resume routes. Independent local work proceeds only when safe.
  Changed saved permissions return HTTP 422 through both verification and resume;
  the original checkpoint remains usable. Nested Agent receipt drift yields a
  diagnostic plan without retry advice. No additional Agent request occurs during
  checkpoint/resume calls. This is not a live Rust Agent or physical calculation.
- The existing result-payload-independence and successful-blocker API controls are
  included in the expanded suite. Preparation-only and draft checkpoints retain
  their policy, but do not claim verified execution recovery.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68120
```

Final service result: 2,612 total, 2,541 passed, 71 conditional skips, zero failures.
Seed 68119 also passed before the final inner-Agent-receipt checks; seed 68120 is
the final source rerun. Conditional skips are not executed evidence. Mix required
local TCP test sockets; no remote server or deployed service was used.

Formatting, whitespace, document-book/inventory, tensor self-test/validation and
project organization checks pass. New files and edited sources/documents remain
within the 800-line source and 2,000-line document limits. The tensor still records
13 modules, 11 paradigms, zero structural gaps, four maturity gaps, 20 evidence-grade
gaps and 15 P0 gaps; release qualification remains blocked.

## Compatibility and limits

Legacy execution checkpoints without recovery evidence cannot authorize reuse.
Regenerate explicitly from original results; do not fabricate evidence from a
boolean or silently replay tasks. The v1 schema describes new output fields
without claiming that older evidence-free execution records are certified.

This is control-plane consistency, not authenticated execution provenance. A party
able to rewrite all receipts, proof and checksums consistently is not authenticated
by these fingerprints. Current attempt ownership, trusted receipt provenance,
producer readiness retention and durable restart remain separately scoped work.
Diagnostic storage does not certify every original failure field as true.

No new native Engine test coverage is claimed. Numerical algorithms and the
Engine/Solver split are unchanged; no installed, remote, physical, scale or general
durable-recovery qualification is implied. This adds one scoped evidence claim,
not an increase in module maturity or a release qualification upgrade.
