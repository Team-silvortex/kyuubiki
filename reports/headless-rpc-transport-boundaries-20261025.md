# Direct SDK RPC Transport Boundaries

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
Follow-up to [lost admission recovery](headless-durable-admission-recovery-20261025.md).
No package update, commit, installation or remote deployment.

This is bounded source-built SDK/Agent verification, not installed or general modal qualification.

## Reproduced Defects

Three new regressions failed against the previous standalone Rust SDK:

1. A successful response naming another request was accepted as our result.
2. A response with both a result and an error was accepted when `ok` was true.
3. In this run, 64 consecutive calls generated only one distinct request ID.
   `SystemTime::now().elapsed()` measured clock resolution, not a unique instant.

Inspection also found unbounded response-header allocation, unchecked request
length narrowing, unlimited retained progress messages and per-read timeouts
that could be extended by partial traffic. No multi-gigabyte allocation was
attempted to reproduce these resource risks; header-only rejection is tested
after the guard exists.

## Implementation

`SolverRpcClient` remains the same public direct-Agent interface. Its transport
and wire checks are separated into small SDK-private modules, with no new
Engine, Solver or repository Protocol dependency. Request IDs include process
identity, an epoch value and an atomic process-wide sequence with checked
exhaustion. The sequence, not clock precision, prevents same-process collisions.
IDs are correlation identifiers, not cryptographic credentials.

Every final response and progress frame must match the actual request ID and
RPC version 1. Typed envelope decoding rejects duplicate known fields, invalid
types, contradictory result/error states and mixed progress/final states.
Progress validates the existing stage, job-ID, finite range, residual and
counter rules. Valid errors retain code/message; progress never becomes a
terminal result. Terminal data are decoded without cloning a complete result
out of an already materialized outer JSON document.

The caller can inspect `limits()` or set a validated `SolverRpcLimits` through
`with_limits`. Defaults are 256 MiB per encoded request/response payload,
4 MiB cumulative encoded progress payloads, 4,096 progress frames, 15-second
idle I/O and a shared 30-minute network deadline. Request serialization is
bounded before connecting. Incoming lengths are validated before allocating
a body buffer. Progress budgets are cumulative, not merely per-frame limits.
Byte-at-a-time traffic and heartbeats do not restart the total deadline.
Zero progress budgets reject progress instead of silently dropping it.

Limits are explicit caller policy, not a solver tolerance or negotiated Agent
capability. Large jobs may use larger reviewed budgets subject to Agent limits.
The SDK does not silently increase limits, resend, cancel or mark an execution
failed after a timeout/protocol error. Durable callers inspect their original
intent/receipt; ordinary direct calls may have an unknown server-side outcome.
Closing a connection may still trigger Agent-side disconnect handling; not
sending a cancellation RPC does not promise that the Agent continues the solve.

## Checks

The dedicated transport suite covers foreign identities, bad versions/types,
contradictory success, malformed progress, duplicate fields, invalid JSON,
request-ID uniqueness across clients, invalid policy, pre-connection request
limits, empty/oversized headers, partial-header/body EOF, cumulative progress
limits, exact byte/count boundaries, total deadlines under heartbeats and
partial headers, real error preservation, and adjacent-float/signed-zero
readback. Its temporary loopback peers are fault fixtures, not physical solvers.

A real-Agent direct modal test separately validates production progress/final
frames, independently checks all four physical modes and confirms an invalid
density returns a real RPC failure. The nine existing actual-Agent durable
recovery cases are retained, including lost replies and process termination.
The legacy session smoke peer was missing required protocol version/request ID;
its fixture now emits the actual protocol envelope instead of weakening checks.

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

## Verified Results

- Dedicated transport regressions: 13 passed. The three originally failing
  identity/state/ID regressions now pass without weakening the checks.
- Full SDK all-target suite: 141 passed, 0 failed, 10 opt-in live tests ignored.
  All 10 live tests were explicitly executed by the focused target; do not
  count ignored tests as execution or add overlapping unit counts twice.
- `modal-sdk-durable-research` ran the actual command with `executed=true` and
  passed. Its retained local report is
  `tmp/headless-rpc-transport-validation-20261025.json`.
- SDK all-target Clippy with warnings denied, changed-file formatting,
  documentation book/inventory, repository organization and diff checks passed.
- Tensor structure passes with 0 structural, 2 maturity, 23 evidence-grade and
  18 P0 gaps. Acceptance remains `blocked`; this evidence strengthens the
  existing verified SDK journey rather than closing production obligations.

## Scope And Limits

These are encoded-byte budgets, not exact parsed-JSON memory or CPU limits.
Synchronous OS hostname resolution cannot be forcibly interrupted; its elapsed
time is counted before subsequent I/O. Request encoding precedes the network
deadline. The client deadline does not establish a solver execution deadline.
Raw RPC still requires a trusted endpoint and adds no TLS/authentication.
This run does not qualify remote/installed services, Python/Elixir parity,
arbitrary physical models, large-load performance or host-power-loss safety.
Numerical algorithms, publication tolerances and production acceptance gaps
are unchanged. Local temporary peers and Agents do not operate installed services.
