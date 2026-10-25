# SDK Failure Context After Timeout And Restart

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [RPC transport boundaries](headless-rpc-transport-boundaries-20261025.md).
No package update, commit, installation or remote deployment.

This is bounded source-built SDK/Agent verification, not installed or general modal qualification.

## Reproduced Defects

The initial durable receipt suite produced seven passes and three failures
against the preceding implementation. The added negative controls demonstrated:

1. A valid outer observation could carry an accepted terminal response with an
   invalid inner RPC version.
2. A failed terminal response with an empty error object was accepted as a
   valid receipt without the original failure code or message.
3. Reading a valid failed receipt returned a generic incomplete-execution error,
   losing the actual solver reason at the public SDK result boundary.

Inspection also found that direct RPC decoding discarded structured error
details and validated response identity when constructing `SdkError::Rpc`.
The Agent already retained the original failure; this repair is SDK-side and
does not change a solver algorithm, tolerance or Agent execution policy.

## Contract Changes

`SdkError::Rpc` now includes optional `request_id` and `details`. Valid direct
remote errors retain both from the validated response. Local configuration
errors set both to `None`, rather than inventing a remote execution identity.
Use `..` for fields not consumed by a match; callers manually constructing
the public variant must supply the new fields.

Durable terminal receipts independently validate inner RPC version 1, original
request identity, status/ok agreement, exclusive result/error state and absence
of active progress data. Failures require non-empty string code and message.
Retained receipts require an explicit response state. Handle identity rejects
whitespace-only or control-character values as well as oversized identities.

After full receipt/handle validation, `completed_result` returns an original
`SdkError::Rpc` for a failed solve, including optional diagnostics. It does not
return a result, treat an outer successful inspection as solver success or
grant resubmission. Foreign or malformed receipts remain validation errors;
their failure context is not accepted as belonging to the original execution.
Diagnostic hints, including nested completed states or replay claims, remain
opaque data and cannot change the terminal state or authorize execution.

Diagnostics can contain sensitive or untrusted data. `Display` includes only
code/message; derived `Debug` includes details. This is not general redaction:
callers own access control, logging and retention for all remote text, raw
receipts and structured details. No credential-storage policy is introduced.

## Real Failure Journey

A test-only relay withholds the actual admission response from a real native
Agent. The public SDK uses a 100 ms idle timeout; the relay holds the response
for 400 ms. The SDK receives `SdkError::Timeout`, not a fabricated solver reply.
The harness observes the actual admission separately, waits for the retained
negative-density solver failure, then kills and restarts that Agent with the
same durable store. A read-only inspection using the caller's saved intent
recovers exactly the original failed receipt. Its original request ID, code,
message and structured details remain available, and the relay observes exactly
one submission. No admission or terminal response is synthesized by the relay.

The prior lost-admission failure test now also checks the actual diagnostic
context after restart. A separate direct modal test verifies the live response
identity and reason code agree with the Agent failure details. Successful
four-mode physical readback, interrupted/unknown outcomes, missing admission,
store replacement and corrupted local admission controls remain in the suite.

This verifies process-restart readback after a committed failure, not that an
in-flight solve continues after a disconnect or that all timeouts mean failure.
It does not simulate host power loss or make interrupted work resumable.

## Verification

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

- Fresh SDK all-target suite: 148 passed, zero failed, 11 opt-in live tests ignored.
- Focused recovery target: 12 durable receipt, three intent, 15 transport and
  one shared digest-contract test passed. The digest contract covers 14 vectors.
- All 11 real-Agent tests were explicitly executed and passed: six admission
  fault/recovery tests and five modal/research tests. Ignored default tests are
  not execution evidence; overlapping suites must not be added together.
- All-target SDK Clippy with warnings denied passed.
- The validation framework reran `modal-sdk-durable-research` successfully with
  `executed=true`; its local machine report is retained at
  `tmp/headless-failure-context-validation-20261025.json`.
- Changed-file formatting, diff whitespace, documentation book/inventory and
  project organization checks passed. Source/doc limits remain 800/2000 lines.
- The tensor checker passed structurally: 13 modules, 11 paradigms, zero
  structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.
  Daji acceptance remains `blocked`; this is not a release qualification.

## Scope

The existing SDK-headless contract/execution/recovery evidence remains
`verified`, not `qualified`. This strengthens one bounded local research
journey; it does not establish installed or authenticated remote acceptance,
Python/Elixir parity, arbitrary-model numerical validity, large-load performance
or host-power-loss durability. Engine/Solver separation is unchanged. Disposable
test Agents do not manage installed services; their owned temporary stores are
cleaned after the tests.
