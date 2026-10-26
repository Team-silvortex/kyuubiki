# Durable Admission Refusal Without Losing Attempt Identity

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
This extends the official Rust headless SDK's direct-Agent submission contract.
Agent admission policy, execution ledger and Solver numerics are unchanged.
It follows the [store-readiness preflight](headless-durable-store-readiness-20261025.md).

## Reproduced Defect

A real source-built Agent was filled to its advertised 1024-record limit using
small invalid `solve_bar_1d` inputs. These create real failed receipts without a
large numerical solve. A new submission returned the Agent's explicit
`kyuubiki.agent-durable-admission-rejection/v1` report with
`status: not_admitted`, `reason: store_capacity` and `execution_started: false`.

The SDK incorrectly parsed that decision as an execution receipt and returned
`unsupported durable receipt or replay policy`. The red test reproduced this
exact message. Native admission remained safe; the defect lost the caller's
capacity diagnosis and evidence of the explicit refusal.

## Corrected Contract

Both durable submit APIs now return
`SdkError::DurableAdmissionRejected { report }` for a validated refusal. They
still return an ordinary execution report on valid admission or observation.
The distinct error preserves the native reason, message and attempt identity,
and is classified as `FailureClass::Rpc`, not a transport failure.

- Require the exact v1 report shape, current supported `store_capacity` reason,
  explicit non-admission status and false execution/replay flags.
- Bind store, execution, method and typed input digest to the prepared intent.
- Bind the rejection request ID to the actual outgoing RPC attempt. Outer
  response validation alone cannot establish that its inner refusal is current.
- Bound non-empty request/process/message text and reject controls. Unknown or
  contradictory fields, missing fields and future unsupported reasons cannot
  assert known non-admission.
- Preserve ordinary duplicate-observation semantics: an existing receipt can
  retain its original admission request ID even when the ledger is at capacity.
- Keep generic outer RPC error diagnostics distinct. A remote error's details
  cannot manufacture this validated result type.

The private transport now returns its generated request ID to the durable
submission path. Public `call` and `RpcCallOutcome` signatures are unchanged.
Consumers matching every `SdkError` variant must handle the new variant.

This is a decision about one attempt, not a durable terminal record, proof
that an ID never existed, authentication or retry authorization. A refusal
whose reply is lost remains unknown to the client. No automatic reconnection,
polling, resubmission or new task ID is introduced. Caller applications own
retention of the structured diagnostic; the research example surfaces it and
keeps input/intent, but does not write it as a false `admission.json` or create
an additional rejection file automatically.

## Test Evidence

Four ordinary protocol tests cover both submit APIs, exact report preservation,
error classification, foreign/current attempt identity, omitted/null fields,
unknown or contradictory policy/result fields, bounded messages, outer error
context and old-request duplicate observations. The loopback peer observes one
submission and rejects any additional connection.

One new opt-in real-Agent test covers a full ledger end to end:

1. Fill the actual advertised entry limit and wait for failed receipts to settle.
2. Observe the explicit capacity refusal with its original reason and identity;
   the rejected ID is absent and cannot become an execution handle.
3. Fetch the original failed receipt and explicitly observe its duplicate
   submission without allocating another execution or changing the old receipt.
4. Drop a different submission's real refusal reply through the bounded relay.
   The SDK returns transport uncertainty; only the relay knows it was refused.
   A deliberate original-ID inspection returns `not_found`, with one dispatch.
5. Kill and restart this isolated Agent. The original terminal receipt remains
   identical, both refused IDs remain absent, record count stays at the limit,
   and no execution remains active. No live service or installed ledger is used.

The focused red/green run passed all six entries: four protocol tests, the live
capacity test and the existing relay socket-inheritance regression. This is
entry-capacity coverage, not byte-budget exhaustion, disk-full injection,
engine-concurrency rejection, authenticated remote or host-power-loss testing.

## Validation

- SDK all-target tests: 190 passed, zero failures, 33 ignored in that invocation.
  All 33 opt-in tests passed separately through the recovery target below.
- The `modal-sdk-durable-research` validation profile executed
  `make test-sdk-modal-recovery` successfully (`executed=true`), including the
  shared protocol digest vectors and all 33 opt-in Agent/protocol tests.
- All-target Clippy with warnings denied: passed.
- Targeted Rust formatting and `git diff --check`: passed.
- Documentation book/inventory and project organization checks: passed, with
  800-line source / 2000-line documentation limits and zero tracked debt.
- Tensor structure: passed with zero structural gaps. Overall daji status remains
  `blocked`, with 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

Runs overlap and must not be added together. Scope remains bounded local
source-built SDK/Agent verification, not installed or general modal qualification.
No version bump, commit/push, installation or deployment is included.
