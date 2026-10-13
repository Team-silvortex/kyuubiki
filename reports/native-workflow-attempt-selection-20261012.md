# Native workflow attempt selection and observation revalidation

Date: 2026-10-12. Daji 3.6.x local source follow-up based on `f45f1e1f` and
the existing uncommitted task-batch recovery repairs. No version change, commit,
push, packaging, installation or remote execution.

## Failure and repair

Native node recovery previously searched retained records for a successful
observation and selected the first match. The same parent/task could have another
failed, blocked or successful attempt, yet a historical success still supplied its
output. Journal ordering uses wall-clock time and an opaque attempt ID; neither
establishes which of those computations is authoritative. Retrieval also did not
recheck the selected observation after waiting for its receipt.

- A success competing with any other dispatched attempt for the same parent/task
  now blocks with `dispatch_attempt_ambiguous`. Time ordering, input equality and
  a recoverable receipt do not resolve that ambiguity. Confirmed `not_dispatched`
  records are excluded from competing attempts. Existing unknown-outcome, input
  identity and unbound-ambiguity checks retain precedence.
- After validating the original Agent receipt, recovery rereads the journal and
  repeats selection. The entire selected observation must still match. A changed
  state or evicted selected attempt blocks with `dispatch_observation_changed`;
  a new relevant attempt or unavailable journal preserves its own blocking reason.
  It does not choose another success, probe a peer or return fresh-execution advice.
- Claim/lease gates still bracket receipt recovery. Both new reasons enter the
  existing blocked-recovery transition, retaining the original envelope, generation
  and attempt. The graph's skip-on-error policy cannot turn them into partial success.
- A single owned success remains reusable alongside confirmed non-dispatched
  records and unrelated parents. History without any success retains the existing
  dispatch policy and ownership gates. Explicit historical receipt inspection is
  unchanged; inspecting an attempt is not permission to attach it to a workflow.

This adds a bounded journal read, not a new archive, backup, larger retention budget
or copied numerical payload. The existing journal limit remains 512 records with
128 retained terminal observations. No compilation or physics moves into Orchestra.

## Verification

The initial new selection contained 17 tests: 15 failed against the old code,
returning a retained output where a block was expected; two isolation controls
passed. These cover related behaviors, not fifteen independent vulnerabilities.
The repaired focused selection passed 94 tests before the additional eviction
case. The final addition has 18 attempt tests plus two coordinator reason cases.

- Same-parent success/success, success/failure and success/blocker combinations
  are tested in both orders where applicable. Refusal occurs before any receipt
  probe. Journal process restart preserves the refusal and original file bytes.
- During a controlled receipt probe, the selected state changes, another success
  or failure appears, or an unknown dispatch is added. All block output reuse
  without probing another attempt. Losing the journal during the read also blocks.
- A real bounded-retention test ages the selected fixture, then appends 128
  unrelated terminal observations during retrieval. The selected file is evicted;
  the already obtained receipt cannot recreate it or authorize stale reuse.
- Foreign-parent and unbound successful records do not resolve an owned ambiguity.
  Conversely, an unrelated parent's new unknown attempt does not poison a uniquely
  owned success. Confirmed non-dispatches are not treated as competing computations.
- Existing native adapter tests retain all eleven bounded diagnostic/scoring
  operators, original endpoint selection, input/consumed-path binding, unavailable
  receipts and claim checks. The previous fallback test now uses a foreign-parent
  historical success because multiple owned attempts stop earlier by design.
- Coordinator tests retain the envelope and block repeated recovery for each new
  reason. The graph checks both reasons under skip-on-error; no dependent export,
  solve or transform is invoked. Existing stale-claim and lease-takeover checks
  remain in the expanded suite.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68123
```

Expanded service result: 2,632 total, 2,561 passed, 71 conditional skips, zero
failures. Conditional skips are not executed evidence. Tests use controlled
receipt responses and real local journal files/process restarts, not a live Rust
Agent, whole-machine reboot or new numerical study.

The adjacent dispatch journal, original-receipt, cancellation, parent-identity and
headless recovery-gate selection also passes 35 tests, zero failures:

```sh
mix test test/kyuubiki_web/orchestra/operator_dispatch*test.exs test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs --seed 68124
```

Formatting, whitespace, document-book/inventory, tensor self-test/validation and
project organization checks pass, with zero tracked file-size debt under the
800-line source and 2,000-line document limits. The tensor remains at 13 modules,
11 paradigms, zero structural gaps, four maturity gaps, 20 evidence-grade gaps and
15 P0 gaps; release qualification stays blocked.

## Limits and next boundary

No new native Engine test coverage is claimed. No installed/remote, physical,
scale, power-loss or general durable-recovery qualification is implied. The
Engine/Solver boundary, Agent protocol and published SDK interfaces are unchanged.

Revalidation is a point-in-time check, not an atomic transaction spanning the
journal, Agent and final job publication. It does not detect every change-and-revert
sequence or authenticate a consistently rewritten journal. Explicit node-to-attempt
lineage, immutable/monotonic observations and cross-store publication fencing remain
separate work. Confirmed history that aged out before selection is still governed
by existing replay policy; this round covers loss during a selected read, not an
unbounded historical ledger. Existing missing/unknown outcome restrictions remain.

An ambiguous original job remains blocked for inspection or explicit resubmission;
do not delete inconvenient records or silently downgrade its recovery claim. The
tensor adds one bounded Orchestra evidence claim without increasing module maturity
or release qualification.

Subsequent [observation-transition work](operator-dispatch-observation-transitions-20261012.md)
now seals the first observation through the journal API and makes repeats read-only.
The observation-replacement tests above retain fault injection outside that API;
normal late conflicting finishes are rejected. Cross-store atomicity and provenance
remain outside this follow-up; the counts above describe the earlier test run.
