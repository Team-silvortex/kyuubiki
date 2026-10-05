# Official Rust SDK Task Completion Verification

Reviewed on 2026-10-05 against the daji 3.4.7 working tree based on commit
`2ef0cb4b`. This round extends completion checks from the native batch executor
to the separately packaged official Rust Headless SDK's model plan execution.
It preserves raw transport APIs, the existing v2 execution receipt, and the
separation between the SDK, Orchestra, and Agent calculation engine.

Local official Rust SDK verification only; not deployed or cross-language qualification.

## Reproduced Failure

The new regression target initially passed two tests and failed seven. A custom
dispatcher returning an HTTP-shaped blocked, failed, stale, or partial task
receipt was treated as completed and allowed the next action. A nested Agent
pending state was ignored when the enclosing Orchestra receipt said executed.
The prior native executor fixes did not protect this separate SDK package.

## Completion and Recovery Contract

- Model plans verify single-task receipt identity and publication. Present
  provenance/validation mirrors and digest-verification flags must agree.
  Nested Agent TaskIR state is checked independently of outer dispatch success.
- Batch completion requires the submitted batch digest, supported execution
  contract, complete task/executed/ok counts, zero errors, unique case coverage,
  and matching per-case identity and readiness. Reordered results are allowed;
  partial, skipped, duplicate, stale, malformed, or contradictory results halt.
- Both Rust completion gates reject executed readiness that still declares a
  blocking stage, blocking reason, or required repair action.
- A noncomplete model step stops downstream actions, does not increment
  completed steps, and retains its authority and unmodified output beside the
  bounded error. Error descriptions do not echo rejected identity/error values.
  Scientific result values, including negative zero, remain unchanged.
- The plan uses the existing v2 failed status. It can enter a blocked research
  frontier after caller verification and JSON save/reload. Readiness repair
  actions are exposed through the existing deduplicated recovery helper.
- Explicit later execution can recover after the gate is repaired. There is
  no automatic resubmission, approval bypass, or promotion of preparation to
  synchronous calculation. Low-level HTTP methods still expose raw receipts.

## Verification

Run from the repository root with the pinned toolchain and offline lockfiles:

```text
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline
cargo clippy --manifest-path sdks/rust/Cargo.toml --all-targets --locked --offline -- -D warnings
```

The official Rust suite passed 98 tests, zero failures, zero ignored tests.
Eleven new tests include receipt mutation matrices, batch identity/count gates,
successful full-result preservation, preparation behavior, retained blockers,
frontier integration, and explicit recovery. The target is automatically part
of the existing Cargo suite, not a manually isolated validation entrypoint.
Three preexisting strict Clippy formatting warnings in SDK workflow examples
were corrected; all SDK targets then passed strict Clippy.

`session_http_task_gate_retains_blockers_and_recovers_with_fresh_execution`
uses the real official `KyuubikiSession` and `SessionModelActionDispatcher`
over loopback HTTP. Its bounded test server sends split bodies and checks task
payloads. The blocked attempt issues only health and task requests, then stops.
An explicit second attempt completes and reaches the next catalog action. It
uses controlled receipt fixtures, not an actual deployed Orchestra or solver.

The native Headless SDK suite is also rerun for the additional contradictory
readiness cases: 280 tests passed, zero failures, zero ignored tests. Native and
official SDK all-targets strict Clippy, changed-file formatting, diff whitespace,
documentation book/inventory, and source/document organization checks passed.
The tensor structure and command checks passed without closing its 14 P0 gaps.
The separate native Agent-backed evidence and aggregate-run limitation
remain recorded in [native completion verification](native-headless-task-completion-20261005.md).

## Scope and Remaining Work

The tensor claim is bounded to local Rust SDK execution, contract, and recovery,
at verified grade. It does not raise SDK parity, installed, operational, or
numerical qualification. TaskIR admission and actual numerical correctness are
still runtime responsibilities and require their own evidence.

The client gate does not itself repair the backend producer. A subsequent
local backend round now validates Orchestra envelope/batch completion and
checkpoint recovery; its scope and results are recorded in
[Orchestra completion verification](orchestra-task-completion-20261005.md).
Python/Elixir model-plan completion gates, authenticated deployment acceptance,
and sustained remote physical research remain separate work.

No new dependencies, version bump, commit, push, installer changes, or release
rebuild were performed. Product version remains daji 3.4.7.
