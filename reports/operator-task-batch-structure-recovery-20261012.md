# Direct task-batch structure and recovery identity admission

Date: 2026-10-12. Daji 3.6.0 source working-tree overlay at `0e1ac6cf`,
following direct execution-batch quality admission. No version bump, commit,
packaging, installation or remote execution.

## Failure and repair

Direct batch validation traversed nested caller-owned values before checking
their types. Malformed task/operator/program objects could raise rather than
return a controlled rejection. Recovery also deduplicated case identifiers,
while a batch could contain duplicate case or actual TaskIR identifiers. This
could merge distinct work in a retry plan. Checkpoint input shape and target
membership were not consistently checked at creation and verification.

- One shared control-plane contract validates the batch, entries and the TaskIR
  containers used by admission and summaries before preparation or execution.
  Supplied object containers must be string-keyed objects; relevant text leaves
  must be text or null. This does not recursively scan or clone numerical models.
- Every entry requires a nonblank case ID and actual nested TaskIR task ID.
  Both are unique within the batch. IDs are not silently generated, trimmed or
  deduplicated. Optional count, operator, RPC-method and case-index headers must
  agree with the tasks. Recovery indexes are derived from those tasks.
- Checkpoint creation, verification and resume planning reuse batch validation.
  Preparation/execution summaries must be objects or absent. Supplied recovery
  target lists contain unique case IDs belonging to this batch. Receipt recovery
  objects, supplied receipt case IDs, action arrays and checkpoint metadata receive
  bounded shape/membership checks before policy evaluation.
- Verification checks supported status/action pairs and compares the declared
  resume policy with the policy derived from the checkpoint summaries. Rehashing
  a malformed or independently rewritten policy does not make it acceptable.
- Public routes return HTTP 422 with location-qualified structural or identity
  errors. The valid-input map return of the low-level checkpoint helper remains;
  invalid input returns an error tuple. The public envelope uses its checked form.
- Empty batches, omitted optional headers and nullable optional text leaves remain
  supported. Missing optional semantic containers still reach ordinary task
  validation; this is not a new requirement that every task already be runnable.
  Digest/semantic failures remain isolated per task under the existing policy.

An older digest-failure fixture reused the first task's ID for its second entry.
It now creates an independently identified second task before corrupting its
payload. Its original per-task failure assertions remain, with duplicate identity
covered separately rather than weakening the new gate.

## Verification

- Twenty-three direct tests cover 14 structural path families, duplicate/blank
  identities, malformed checkpoint summaries and receipts, foreign or repeated
  recovery targets, rehashed bad policies and valid recovery controls. Where the
  malformed TaskIR still supports hashing, both stale and recomputed digests are
  tested so the shape check does not rely on incidental digest rejection.
- Two API tests include 20 invalid batch requests across preparation, execution,
  checkpoint creation, checkpoint verification and resume planning. Six malformed
  checkpoint requests also return HTTP 422. A corrected batch prepares, executes
  two local export tasks and reaches an archive plan; its earlier valid draft
  checkpoint still verifies.
- Process-local tracing checks 16 invalid batch controls: malformed TaskIR,
  duplicate case ID, duplicate TaskIR ID and invalid quality evidence, each in
  either position with strict mode on/off. They produce zero task-execution
  calls; the valid control produces two. The trace delivery barrier and cleanup
  are retained. These are actual local executor calls, not a live Agent test.
- A digest-invalid task remains individually failed while its peer completes;
  checkpoint targets retain the unique failed case. A corrected fresh batch
  completes without silently reusing ambiguous recovery identities.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68108
```

Expanded service result: 2,583 total, 2,512 passed, 71 conditional skips, zero
failures. The earlier focused selection passed 139 tests before the API additions.
Mix required permission for its local TCP synchronization socket. Conditional
skips are not executed evidence. No new native Engine test coverage is claimed.

Formatting and diff-whitespace checks pass. Document-book and inventory checks,
tensor self-test/validation and the project organization audit also pass. Source
files remain within 800 lines and documents within 2,000, with zero tracked size
debt. The tensor still records four maturity, 20 evidence-grade and 15 P0 gaps;
release qualification remains blocked rather than being promoted by this repair.

## Scope and remaining work

This is control-plane source admission and explicit fresh-run recovery, not
physical accuracy, live Agent, installed or durable-recovery qualification.
Structural consistency and checksum agreement do not authenticate execution
evidence. A summary still needs stronger counter coherence, receipt-to-task
binding and provenance checks; target membership alone does not prove that a
target really failed or that a preparation summary is complete.

Standalone TaskIR entrypoints, remaining per-entry RPC fields and producer-side
readiness retention through ranking are separate audit boundaries. Legacy
checkpoint JSON normalization is unchanged; this is not a generic lossless JSON
identity upgrade. Numerical Engine/Agent contracts and algorithms are unchanged.
These scoped checks do not qualify an entire module or clear release blockers.
