# Durable SDK Admission Loss And Input Binding

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [durable modal research](headless-durable-modal-research-20261025.md).
No package version change, installed application update or remote deployment.

This is bounded source-built SDK/Agent verification, not installed or general modal qualification.

## Reproduced Gap

The earlier SDK checked admission store/execution IDs and method but did not
compare the returned request digest with the input it actually submitted.
`matching_ids_cannot_hide_a_receipt_for_different_input` failed against that
implementation: an otherwise well-formed receipt with a different digest was
accepted. The same regression passes with the input-binding fix.

Before sending, `DurableExecutionIntent::new` records the original store, task
ID, method and typed input digest. The caller persists that intent, the exact
input and trusted endpoint, then calls `submit_prepared_durable_agent_task`.
Mutated input refuses before connection. Both the prepared and convenience
submit APIs compare the actual admission with the input digest. Neither API
silently persists caller data or grants retry authority.

After a transport error, `inspect_durable_agent_task` only reads the original
store/task and validates content identity. A matching completed receipt can
provide a handle even when admission acknowledgement was lost. A present
admission still pins the original request/process identity. Corrupt or
contradictory saved admission cannot fall back to weaker intent-only recovery.
Older intents without a digest are rejected, not backfilled from the observed
receipt. `not_found`, `outcome_unknown`, `running` and `failed` are not results
and never authorize automatic resubmission.

## Contract And Architectural Boundary

- The standalone SDK has no Solver, Engine or repository Protocol dependency.
  It implements the existing typed durable digest contract, not a new solver.
- Fourteen shared fixed golden vectors are independently consumed by SDK and
  Agent protocol tests: object ordering, array ordering, method binding, Unicode,
  integer limits, integers versus floats, signed zero, adjacent binary64 values,
  subnormal/maximum finite values and execute-mode TaskIR input.
- The SDK suite repeats the contract after JSON serialization/readback and
  rejects malformed intents, excessive nesting and non-execute TaskIR mode.
- This is content/identity consistency, not signed attestation or authentication.
  Caller-owned storage and endpoint trust remain assumptions. No claim of
  cross-language digest support, arbitrary TaskIR execution or new exchange format.

## Real Fault Injection

`durable_modal_ack_loss_live` uses a bounded native loopback relay. It forwards
real request/response bytes to the production Agent and drops the first actual
admission reply after the Agent sends it. Only the test harness observes that
reply; SDK recovery uses the saved intent. There are no fabricated Agent receipts.
The harness records every request method and asserts exactly one submission.

1. A solve completes after its acknowledgement is lost. Kill/restart the Agent,
   read the same completed receipt and independently recheck all four physical
   modes. Changed input under the same ID rejects the original receipt.
2. Hold a real modal solve at `modal_shape_norm`, lose acknowledgement, observe
   the hold, then kill/restart. Repeated reads stay `outcome_unknown`, without
   modes or any active execution, even after removing the hold file.
3. Drop the request before forwarding to the Agent. After restart, repeated
   reads stay `not_found`; the ledger remains empty and no retry is sent.
4. A solve with invalid negative density fails after lost acknowledgement.
   Restart preserves the failed receipt and reason; it cannot publish a result.
5. Run the SDK example as separate submit and inspect processes. Submit exits
   with transport failure after retaining input/intent but no admission file.
   Following Agent restart, inspect exports the original checked result. A
   corrupt admission, contradictory process identity or changed intent digest
   blocks later export and does not alter previously verified evidence.

The prior four real-Agent tests still cover normal acknowledged recovery,
reversed numbering, ledger replacement, interrupted-work isolation and disk
readback negative controls. They remain in the same reproducible target.
All process kills target owned temporary test Agents only; installed services
are untouched. Process termination and file syncing are not host-power-loss
qualification. Test data are cleaned up after successful runs.

## Test Harness Follow-Up

The first SDK all-target run exposed an existing parallel-fixture collision in
the layered thermal example: process ID plus timestamp was not unique at the
host clock's resolution (`AlreadyExists`). Its test-only directory helper now
adds an atomic sequence, with eight simultaneous fixtures at a fixed timestamp
as a regression. The real-Agent helper uses the same uniqueness precaution.
No physical model, production algorithm or tolerance was weakened to pass.

## Reproduction

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

The focused target builds the actual Agent and SDK example, runs the Protocol
golden test and SDK boundary tests, then explicitly runs both opt-in live suites.
Local socket/process permissions are required. A default SDK run ignores the
live suites; those skips are not evidence of live execution.

## Verified Results

- Focused target: 1 Agent Protocol test (14 golden vectors), 10 SDK boundary
  tests and all 9 real-Agent tests passed, including the 5 new fault cases.
- After fixing the test-directory collision, the complete SDK all-target suite
  passed 128 tests with 0 failures and 9 opt-in live tests ignored. The 9 live
  tests were executed separately above. The 10 boundary tests are already part
  of the 128 and must not be added again.
- The `modal-sdk-durable-research` profile executed its actual test command and
  passed with `executed=true`; its local result is
  `tmp/headless-durable-admission-validation-20261025.json`.
- SDK all-target Clippy and the Protocol fixture-target Clippy passed with
  warnings denied. Changed-file formatting, documentation book/inventory,
  project organization and diff whitespace checks passed.
- The tensor remains at 0 structural, 2 maturity, 23 evidence-grade and 18 P0
  gaps, with Daji acceptance `blocked`. This follow-up strengthens bounded
  evidence and does not relabel the remaining production obligations.

## Remaining Acceptance

The evidence extends the existing `sdk-headless` / `sdk_headless` contract,
execution and recovery coordinate at `verified`, not `qualified`. It does not
close installed Python/Elixir journeys, heterogeneous long-bending publication,
six-mode recovery, authenticated Mesh/Orchestra deployment, multi-host faults,
Windows/Linux execution or whole-machine power loss. The small independent
four-mode reference is unchanged and remains specific to its explicit inputs.
