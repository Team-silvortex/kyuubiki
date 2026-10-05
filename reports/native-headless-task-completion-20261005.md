# Native Headless Task Completion Verification

Reviewed on 2026-10-05 against the daji 3.4.7 working tree based on commit
`2ef0cb4b`. The production native batch and service executor now distinguish
request delivery from actual task completion. This report covers false-success
prevention, TaskIR identity checks, downstream isolation, and bounded local
Agent recovery; it does not qualify new physical solver families.

Local native batch verification only; not installed or cross-language qualification.

## Reproduced Defects

Three new regressions failed before the fixes: a custom executor still received
digest-tampered TaskIR, blocked executor outcomes were counted as success and
allowed the next side effect, and mock TaskIR previews were reported as executed.
Inspection also found the CLI returned success for a blocked execution report.
The previous HTTP readiness test explicitly expected `executed` despite a
blocked runtime receipt; it now uses a valid digest-bound task and expects
`blocked` while retaining the original readiness, fetch request, and plan.

## Corrected Contract

- Task execution verifies digest, authoring/ABI mirrors, and admission before
  reaching a custom executor or opening a service connection.
- Service completion replies match the submitted task id, task digest, operator
  id, and program id. Present validation/provenance identity mirrors are checked.
- An outer Orchestra dispatch-success envelope cannot promote a pending or
  blocked nested Agent TaskIR receipt into executed computation.
- Missing publication, unknown completion/readiness states, explicit failure,
  stale identity, and contradictory gates fail closed. Error messages identify
  the invalid field without reflecting the rejected values.
  Executed readiness must not retain a blocking stage, blocking reason, or
  required repair action; the HTTP mutation matrix covers each contradiction.
- Only an `executed` executor outcome contributes to successful step count and
  downstream bindings. Blocked, failed, cancelled, and unknown outcomes halt the
  batch before subsequent side effects. Structured blocking receipts survive.
- Mock TaskIR execution is a blocked preview. Explicit blocked CLI execution
  writes the requested run report, then exits unsuccessfully with stable JSON
  error code `headless_execution_blocked`, stage `execution`, retryable false.
  Confirmation blocking has the same non-success execution semantics; dry-run
  remains a separate planning operation.
- Successful full results remain available for bindings; report-only array
  compaction does not truncate the input to a later step.

## Real Agent Sequence

`headless_http_receipts_from_a_real_agent_block_fail_and_recover` starts the
development Rust Agent binary, submits each task over its length-prefixed TCP
RPC protocol, waits for its slot to return to accepting, and passes the real
reply through a test-only HTTP envelope adapter to `ServiceHeadlessExecutor`.
The adapter does not calculate or substitute solver outputs.

1. A central-package TaskIR without an attached package runtime yields blocked
   readiness. The SDK does not issue the next project-create operation.
2. A four-element axial bar executes. Its published tip displacement agrees
   with the independent closed form `F L / (E A)` to relative error below 1e-12.
3. The same bar with zero cross-sectional area fails through the Agent engine.
   No successful step or subsequent project-create operation is reported.
4. The healthy bar executes again through the same Agent and SDK executor. Its
   complete task result receipt equals the earlier successful receipt.

This is actual local Agent solver execution, but the HTTP wrapper is a test
adapter, not a deployed Elixir Orchestra. It is not an installed qualification
or evidence for the unresolved longer heterogeneous modal fixtures.

## Verification

Commands run from `workers/rust` with the pinned toolchain and offline locks:

```text
cargo test -p kyuubiki-headless-sdk --lib --locked --offline
cargo test -p kyuubiki-cli --test headless_task_completion --test headless_execution_posture --test headless_parameter_patch --test headless_research_round --test headless_wait_recovery --locked --offline
cargo test -p kyuubiki-cli --bin kyuubiki-headless --locked --offline
cargo test -p kyuubiki-cli --test headless_task_completion native_cli_blocked --locked --offline
cargo clippy -p kyuubiki-headless-sdk -p kyuubiki-cli -p kyuubiki-script-runner --all-targets --locked --offline -- -D warnings
```

The complete native Headless SDK suite passed 280 tests with zero failures and
zero ignored tests. It includes 10 new batch/HTTP completion regressions,
malformed receipt matrices, split HTTP body delivery, same-executor recovery,
pre-request tamper rejection, and the existing full-result binding test.
The real Agent/CLI completion integration target includes three new tests and
two shared Agent lifecycle helper tests; those helpers are not counted as five
independent new completion scenarios. The existing posture, parameter patch,
research round, and wait recovery integration suites are also rerun.
The final socket-free CLI blocking subset passed two tests; the CLI binary
unit suite passed 16 tests. Strict Clippy across Headless, CLI, and script-runner
all targets, formatting, and diff whitespace checks passed. Native tensor,
documentation book/inventory, and project organization checks passed; the
800-line source / 2000-line document limits retain zero tracked debt.

The native `headless-test` entrypoint now includes this completion integration
target, so the regression is not only a one-off manually selected test.
Its aggregate rerun encountered a sandbox `PermissionDenied` while binding
loopback in the existing posture test, before the completion target ran.
Two narrowly scoped approval retries failed in the automatic approval service
before execution. That aggregate attempt is not recorded as a pass. A subsequent
local follow-up, with loopback permission granted, reran the same native
`headless-test --locked --offline` entrypoint successfully: 22 tests passed,
zero failures and zero ignored. The separate `headless-live-test` also passed
four tests against local Orchestra HTTP and controlled Agent fixtures, with
owned child/scratch cleanup; details are in
[Orchestra local verification](orchestra-task-completion-20261005.md).
The earlier
direct Cargo run passed 25 integration tests, including the real Agent sequence
and CLI task receipt; the final confirmation-blocking regression is verified
separately without sockets. CLI-only completion regressions now own temporary
scratch directories and do not start an Agent merely for filesystem storage.
Tensor registration is a bounded `verified` claim for `sdk-headless` execution,
contract, and recovery. It does not raise installed, numerical, or SDK-parity
qualification grades or change the solver admission policy.

## Remaining Gates

The standalone Rust/Python/Elixir clients remain separate client implementations;
this native executor change must not be read as cross-language parity proof.
The separately reviewed official Rust model-plan gate is recorded in
[Rust SDK completion verification](rust-sdk-task-completion-20261005.md);
raw transport APIs and Python/Elixir completion qualification remain separate.
Actual deployed Orchestra HTTP execution, installed packaging, authenticated
remote recovery, sustained workloads, and unresolved modal physical readback
still need their named qualification scenarios. No installed applications or
release artifacts were rebuilt, and no version bump, commit, or push was made
in this verification round.
