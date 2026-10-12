# Direct execution-batch quality admission and fresh recovery

Date: 2026-10-12. Daji 3.6.0 source working-tree overlay at `0e1ac6cf`,
following sweep expansion count admission. No version bump, commit, packaging,
installation or remote execution.

## Failure and repair

The direct `cases` entrypoint checked counts, identities and configuration, but
did not recheck supplied candidate quality metadata. Caller-authored compiled
batches likewise passed preparation/execution despite disabled, unconverged or
incomplete evidence. Top-level quality evidence was discarded by composition
and omitted from the batch digest. The first 30-test selection produced 29
expectation failures before the repair; these are related boundary variants,
not 29 independent defects.

One small control-plane admission helper now reuses the existing quality-plan
intent validator. There is no new numerical algorithm, Rust batch composer or
duplicated native solver implementation.

- Composition validates all supplied batch/candidate quality scopes before
  compiling any TaskIR. Preparation and execution validate them before starting
  a batch run or dispatching any task. A late invalid candidate cannot allow
  earlier tasks to run first, regardless of strict mode.
- Reserved quality fields on the envelope, candidates, `metadata` and
  `case_metadata` follow plan readiness, convergence, completeness and repair
  rules. Disabled expansion and explicit budget-block markers also reject.
  Null or absent metadata does not suppress evidence supplied in another scope.
  Each scope must be admissible independently; different cases are not forced
  to carry one identical report.
- A blocked seed with explicit repair intent remains executable; a warning with
  explicit review intent remains executable. This requests another calculation,
  not a claim that the seed already passed or the research is complete.
- Quality fields survive composition. Top-level quality scopes now participate
  in the batch digest as per-task metadata already did. A changed admissible
  repair declaration fails verification against the checked old checkpoint,
  even with unchanged numerical TaskIR. Old checkpoints whose top-level evidence
  was previously unhashed require explicit regeneration, not silent migration.
  Batches without these optional fields keep the existing digest inputs.
- Ordinary JSON metadata without reserved quality fields remains caller-owned.
  Generic notes, arrays, scalars and null still work. Admission inspects evidence,
  not a cloned or fabricated sweep model. This is not a measured performance claim.
- Ordinary per-task computational errors retain the existing isolation policy:
  other tasks can complete and the checkpoint targets only the failed case.
  Batch quality failures return `operator_task_batch_quality_rejected`, with a
  location-qualified reason. Public prepare/execute routes return HTTP 422 with
  that error code rather than a generic success or unclassified error.

## Verification

- Twenty-six invalid variants cover 130 evidence placements. Each is rejected
  by composition, direct preparation/execution and their public envelope helpers.
  Compiled inputs pass JSON exchange before validation.
- A separate 125-placement cross-scope matrix includes absent, null, valid repair,
  disabled and blocked-without-repair metadata: 27 accepted and 98 rejected.
- Fifteen valid repair/review/ready placements preserve evidence through
  composition, preparation, local analytic execution and checkpoint/resume.
  Five modified-evidence placements invalidate their old checkpoints without
  changing TaskIR. Five ordinary JSON metadata controls remain supported.
- Thirty-two controlled graphs cover eight invalid metadata variants with
  fail/skip policies, each followed by an explicit corrected fresh run. Invalid
  inputs emit no dependent batch/export. Fresh exports retain repair evidence;
  two actual local thermal-shock assessments per corrected graph are checked
  against the independent analytic expression in the shared test helper.
- Process-local call tracing observes the unmocked executor: invalid quality in
  either task position, with strict mode both on and off, produces zero calls to
  task execution. The corrected control produces exactly two calls. A trace
  delivery barrier is used before absence assertions, and tracing is removed
  after the test.
- Two asynchronous API workflow jobs verify failed-history retention and a
  separate corrected job. Compiled evidence corruption gets HTTP 422 from both
  public batch routes; the unchanged corrected batch prepares and executes.
- A digest-bound, well-formed task with invalid numerical input fails individually;
  its peer completes and only the failed case is selected for retry.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68105
```

Expanded service result: 2,558 total, 2,487 passed, 71 conditional skips, zero
failures. Earlier selections passed 2,556 and 2,557 total tests with different
seeds before the final HTTP/matrix additions. Mix required permission for its
local TCP synchronization socket; no remote server or live Agent was used.
Conditional skips are not executed evidence.

Formatting, diff-whitespace, document-book/inventory, tensor self-test/validation
and organization checks pass. The 800-line source and 2,000-line document limits
remain satisfied, with zero tracked size debt. The tensor still records four
maturity, 20 evidence-grade and 15 P0 gaps; release qualification remains blocked.

## Scope and remaining work

This is control-plane source admission and explicit fresh-run recovery, not
physical validation, live Agent, installed or durable-recovery qualification.
No new native Engine test coverage is claimed in this round. TaskIR digest and
Agent execution contracts remain unchanged; the tests do not prove a generic
lossless JSON identity scheme or authenticated quality evidence. Standalone
TaskIR execution does not inherit metadata omitted from its surrounding batch.

Direct-batch structural hardening remains separate: malformed nested TaskIR,
duplicate recovery identities, checkpoint input shape and general receipt/policy
coherence still need dedicated coverage. This gate does not validate every
caller-authored field or recompute physical objectives and sweep estimates.
Producer-side readiness retention through ranking remains another boundary.
The tensor keeps its existing maturity gaps; these scoped checks do not qualify
an entire module or assert complete recovery coverage.
