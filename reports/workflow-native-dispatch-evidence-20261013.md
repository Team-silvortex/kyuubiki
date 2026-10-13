# Durable native workflow dispatch evidence

Date: 2026-10-13. Daji 3.6.x source reliability follow-up.

## Failure and repair

A restarted dispatch journal treats an absent directory as valid first use. That
is necessary for new deployments, but the journal alone cannot prove whether an
existing workflow previously dispatched work. The prior live-cache integrity
checks could not distinguish complete pre-startup loss from an empty history.

Coordinator-managed native diagnostic/scoring nodes now bind that history to a
separate, small field of their existing private workflow recovery record. This
is not a second journal, executable task, result archive or another storage backend.

1. New recovery records initialize a sealed empty `native_dispatch_evidence` bucket.
2. The production TaskIR adapter reserves a normal dispatch attempt in the journal.
3. Before connecting/sending to the Agent, an internal callback commits the exact
   attempt ID and task identity through the coordinator's existing fenced, leased
   job/result transaction. Prior required attempts and the new reservation must
   still match the journal. Failed, malformed, throwing or exiting callbacks send
   no RPC, preserve the unconfirmed reservation and do not penalize Agent health.
4. Recovery admission verifies the bucket and all recorded attempts before saving
   another execution generation. Runtime checks validate the relevant task again
   before reuse and after receipt retrieval. Commit-time revalidation prevents a
   new reservation from concealing history lost after the earlier lookup.
5. An unknown dispatch completion remains recovery-blocked, even with skip-on-error;
   it no longer falls through generic failure cleanup that prunes the evidence.
   Another valid attempt with the same task identity cannot replace a missing ID.

The bucket binds its parent, execution envelope digest and producer claims. Each
entry contains only attempt ID, task ID/digest, operator ID, program ID and claim.
It is capped at 512 entries and 524,288 encoded bytes. There are no task inputs,
results, endpoint addresses, timestamps or credentials. Overflow blocks instead of
evicting required evidence. Graph membership is indexed once per bucket validation,
not rescanned for each entry. Existing transaction storage is reused; public job
responses expose only the policy and count. Ordinary completion/failure/cancellation
prunes the bucket, while recovery-blocked records retain it. No backup is created.

Old native workflows lacking the bucket are deliberately blocked for explicit
inspection/resubmission, not silently backfilled as never dispatched. Pure local
workflows do not acquire a journal dependency. This is control-plane bookkeeping;
no numerical algorithm or Rust Engine responsibility moves into Orchestra.

## Verification

The pre-repair dispatch test expected a failed persistence callback to prevent a
connection. Instead the adapter connected and timed out waiting for a receipt.
The repaired local suite covers:

- Six callback failure forms: error, malformed reply, exception, throw, exit and
  invalid callback. No Agent connection occurs and no raw diagnostic is returned.
- Twelve restart cases across six observation states and absent/empty roots.
  Journal-only empty reads remain valid; the independent workflow evidence blocks
  recovery without increasing its generation or starting another runner.
- Replacement attempts, changed inputs, graph/parent/envelope mismatches, missing
  legacy evidence, malformed sealed records, stale claims and rejected transactions.
- Fresh native/pure local controls, unchanged repeated commits, private public
  projection, ordinary terminal cleanup, entry/byte budgets and malformed journal
  evidence queries that do not poison other requests.
- Two full local service paths: workflow admission to coordinator, Engine adapter,
  production AgentClient/routing/dispatch and a controlled loopback TCP Agent.
  The request's attempt is already committed when the test Agent receives it.
  Normal completion clears temporary evidence; journal loss during the request
  preserves recovery-blocked evidence and suppresses output, even with skip-on-error.
- The new recovery reasons use existing atomic blocking and claim fencing.

Commands run from `apps/web`:

```sh
mix test test/kyuubiki_web/playground/agent_task_dispatch_evidence_test.exs --seed 68133
mix test test/kyuubiki_web/orchestra/workflow_native_dispatch*test.exs test/kyuubiki_web/playground/agent_task_dispatch_evidence_test.exs test/kyuubiki_web/orchestra/workflow_native_recovery_block_test.exs --seed 68138
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/orchestra/operator_task*test.exs test/kyuubiki_web/orchestra/operator_dispatch*test.exs test/kyuubiki_web/orchestra/headless_native_recovery_gate_test.exs test/kyuubiki_web/playground test/kyuubiki_web/library test/kyuubiki_web/storage --seed 68139
```

The first command is the pre-repair failure. The focused repaired selection passed
59 tests. Expanded service regression passed: 2,915 total, 2,844 passed,
71 conditional skips, zero failures. This round adds 43 tests. Conditional skips
do not count as exercised evidence. TCP permission approval required a retry;
no remote service or production Agent was contacted.

Formatting, whitespace, document-book/inventory, tensor self-test/validation and
project organization checks pass. Source and document sizes stay within the
800-line and 2,000-line limits, with zero tracked debt. The tensor remains at 13 modules,
11 paradigms, zero structural gaps, four maturity gaps, 20 evidence-grade gaps and
15 P0 gaps; release qualification remains blocked rather than being promoted by
these source-only regressions.

## Boundaries

This protects managed native workflows only while their private recovery record
survives. It does not retrofit standalone TaskIR calls, legacy solve RPCs or custom
clients that bypass production dispatch. It does not authenticate journal states,
detect a consistent rollback of both stores, guarantee an atomic transaction with
the Agent, or establish exactly-once execution. A crash after journal reservation
but before private evidence commit cannot have sent an RPC on this production path;
the leftover journal reservation still conservatively blocks until inspected.

Journal retention remains 128 terminal records and 512 total records. Active
workflow evidence does not pin terminal history. If required observations age out,
even during a long live workflow, later commit/recovery can conservatively block.
This is deliberate loss detection, not a retention availability solution. Do not
erase evidence or treat another attempt as permission to rerun; active-history
retention coordination remains the next independent boundary to address.

Tests use controlled receipts, not real Rust computation. No installed, physical
power-loss, remote multi-process, PostgreSQL acceptance or numerical accuracy
qualification is claimed. The tensor evidence is scoped to Orchestra contract and
recovery and does not promote module maturity. No version bump, commit, push,
packaging, installation or remote execution was performed.
