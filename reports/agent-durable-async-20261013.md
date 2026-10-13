# Standalone Agent Asynchronous Durable Admission

Date: 2026-10-13. Version: daji 3.6.0. Scope: local macOS source-build tests.

## Change

`submit_durable_agent_task` accepts the existing durable task contract and returns
the admission report without waiting for computation. `run_durable_agent_task`
retains synchronous compatibility. Both use the same ledger and deduplication
identity. Existing entries can be inspected/retried without reserving another slot.

For new work the Agent reserves its existing lifecycle/watchdog/cancellation guard
before syncing admission and spawning the worker. Capacity and drain rejection
leave no new admission record. The detached worker retains that guard through
receipt commit. It owns no TCP reply writer: losing the submit connection cannot
cancel the computation. Shutdown waits for the durable owner, within its existing
bounded shutdown deadline. Spawn failure or unwinding releases the reservation;
an admitted execution without a committed receipt stays unknown, never replayable.

The execution-context seam reuses existing solver and execute-mode TaskIR dispatch.
Operator implementations and engine algorithms are unchanged. No unbounded task
queue, second scheduler, input archive or Node backend is introduced.

The native gateway's `POST /api/direct-mesh/tasks` now forwards to asynchronous
submission. Inspection remains `POST /api/direct-mesh/tasks/inspect` on one pinned
allowlisted Agent. Validation checks producer/request identity, digest shape,
schema, state, nested response identity and terminal evidence; null success results
or contradictory failure results are rejected. IDs and checksums are not auth.

## Frontend Service Contract

The runtime client now exposes `submitDurableDirectStudy` and
`inspectDurableDirectExecution`, with strict typed receipt validation. Study inputs
use the existing UI normalization contract, including GPa-to-Pa conversion for an
axial-bar request. These methods never fetch an Orchestra job or use the old direct
result cache, and never select another Agent after an uncertain response.

`createDurableDirectRecovery` requires caller-provided persistent handle storage
and a stable gateway identity. It snapshots input and awaits handle storage before
sending. The handle contains the execution ID, endpoint, gateway identity, study
kind and schema, not model copies, results or credentials. A storage failure blocks
dispatch. A failed submission exposes the saved handle through
`DurableDirectSubmissionUncertain`. Reconstructed clients inspect the original
handle, including after `not_found` or `outcome_unknown`; they never automatically
resubmit. Gateway changes before sending or while awaiting a response fence the
operation. The storage callback is an integration contract, not filesystem-sync
qualification of any GUI persistence implementation.

## Validation

Real Rust Agent child-process suite: 11 passed, including 9 scenario tests and 2
shared-fixture checks. Temporary files are cleaned; no production services touched.

- Admission returns while the solver is deliberately held. Same-ID duplicates,
  including via the synchronous RPC, do not consume another execution slot.
- Conflicting input and over-capacity admission reject without starting extra work.
- Dropping the submit connection before reading its response does not cancel the
  worker. Its original result survives two forced-kill/restart cycles without replay.
- Killing an admitted, held worker leaves `outcome_unknown` after restart.
- Drain retains the durable owner's slot and rejects fresh work without recording
  admission. Releasing the receipt hold allows quiescence.
- Exact execution cancellation produces a failed receipt retained after restart.
- A receipt-write failure releases capacity, reports unknown, and blocks new durable
  admissions rather than publishing successful computation as durable completion.
- Actual solver TaskIR uses the same reserved guard; bar displacement/stress agree
  with FL/EA and F/A.
- An unconfigured store and nested/control RPCs fail before admission.
- SIGTERM waits for the detached owner; after it finishes, shutdown succeeds and
  the original result can be read after restart.

Frontend targeted suite: 45 passed, including 23 new durable API/recovery checks
and existing study service, controller, client-injection and composer regressions.
The injected HTTP tests verify save-before-send, immutable input snapshot, no
dispatch on storage failure, lost-reply reconnect, gateway fencing and invalid
receipt rejection. They are not installed WebView acceptance tests.

Final scoped regression totals: 435 Rust tests and 45 frontend tests passed,
0 failed. One pre-existing desktop-runtime test remains ignored because it starts
the workstation's real installed stack. Rust totals comprise protocol 142, Agent
unit 199, asynchronous durable live 11, synchronous durable live 11, exact cancel
live 4, lifecycle live 6, and desktop-runtime 62. Clippy passes for all targets of
the three changed Rust crates with warnings denied. The documentation book and
inventory checks, 800-line source / 2,000-line documentation audit, and tensor
structure/command checks pass. The tensor still reports 4 maturity gaps, 20
evidence-grade gaps and 15 P0 gaps; this scoped claim does not clear those gaps.

Reproduce from the repository root:

```text
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-cli --test agent_durable_async_live --test agent_durable_execution_live --test agent_execution_cancel_live --test agent_lifecycle_live
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-desktop-runtime --lib
cargo run --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-script-runner -- frontend-unit-test durable-direct workbench-backend-service-composer workbench-study-run-backend workbench-run-controller workbench-headless-execution-client
```

## Boundaries

Not qualified: physical power loss, remote Linux, Windows directory durability,
installed GUI/PWDT end-to-end recovery, numerical resume, large-result chunking or
full workflow graph execution. The default Workbench/PWDT run/cancel/history route
has not switched to these APIs; migrating those together with visible persistent
handle management remains next work. Installer still does not provision the
opt-in execution store automatically. No deployment, version bump or commit was
performed. Do not count this evidence as closure of the full direct-mode GUI cell.
