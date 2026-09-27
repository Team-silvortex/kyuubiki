# Solver Completion Receipt Reliability

Date: 2026-09-27. Source line: daji 3.4.0. Local development evidence only.

Follow-up: the [atomic publication repair](solver-atomic-publication-20260927.md)
addresses the separate job/result commit described below. Counts and limitations
in this report describe the initial notification/receipt repair, not that follow-up.

## Reproduction

The initial 16-test regression had 15 failures before the repair. It exercised
the shared asynchronous solver submission path, through direct progress calls
and the truss HTTP route using real loopback TCP with scripted Agent responses.
The fixture is not a Rust physical solver and does not establish numerical
accuracy or remote deployment behavior.

Two defects were reproduced:

- An Agent `completed` progress notification made the persisted job terminal
  before its final RPC result arrived. The background runner then skipped result
  storage because the job was already terminal. A later RPC error or disconnect
  could also leave a false completed receipt without a result. A `failed`
  notification could discard a later successful final response in the same way.
- A successful RPC response with no object result reached the result store's
  map-only function and raised `FunctionClauseError`. The background task exited
  while its job remained active, leaving recovery to the later watchdog timeout.

## Initial Receipt Contract

Agent progress describes an execution attempt, not the final control-plane job
receipt. `completed` progress projects to `postprocessing`; `failed` and
`cancelled` notifications retain the active stage while the final RPC response
is awaited. Iteration, residual, and message diagnostics remain available.

Valid incoming progress is capped at 0.99 before monotonic projection, reserving
normal completion for the final response and successful result storage. Existing
higher stored progress is not regressed. Negative, greater-than-one, text, and
null progress remain invalid rather than being clamped into a valid receipt.
The existing failover projection remains monotonic.

The asynchronous runner accepts only a JSON object as the final result. Missing,
null, array, numeric, string, and boolean results produce a failed job with
`invalid solver result: expected a JSON object`. They do not reach the map-only
store function or create a result. This is a transport-shape check, not physical
result-schema validation.

A final RPC error/cancellation or disconnect determines the corresponding
terminal outcome; a progress notification cannot preempt that verdict. A job
already cancelled before the runner's final terminal check keeps its terminal
state and publishes no delayed result. Late progress does not change it. Tests
also exercise subsequent healthy requests after cancellation or malformed data.

## Regression Evidence

- Ten progress tests cover the three terminal-looking notification stages,
  100-percent active progress, invalid ranges/types, late notifications after
  cancellation, preserved diagnostics, and failover projection.
- Thirteen HTTP/TCP tests cover successful result retention, an earlier failed
  notification, RPC error/cancellation, disconnect, five invalid JSON root
  shapes, an absent result field, delayed final delivery, and cancellation
  followed by a healthy request.
- The delayed-response fixture explicitly waits for test release; assertions
  verify the active receipt before allowing the final response. The cancellation
  test waits for its background runner to exit before checking absent results.
- Both the public HTTP receipt and stored result are checked. The same focused
  tests execute against SQLite and the durable memory/JSON backend.

Executed with warnings treated as errors:

- Full local SQLite Web suite: 1014 tests, zero failures, eight existing skips.
- Focused SQLite suite: 23 tests, zero failures for each seed 1, 42, and 1337.
- Focused memory suite: 23 tests, zero failures, no skips.
- Compilation, formatting, diff whitespace, and the 800-line source / 2000-line
  documentation audit pass. Tensor structure/command checks pass; its overall
  maturity gate remains blocked and is not promoted by this evidence.

To repeat from `apps/web`, use a fresh disposable database via
`KYUUBIKI_STORAGE_BACKEND=sqlite` and `SQLITE_DATABASE_PATH`, or a fresh disposable
`KYUUBIKI_DATA_DIR` with `KYUUBIKI_STORAGE_BACKEND=memory`, then run:

```text
mix test --warnings-as-errors test/kyuubiki_web/analysis_solver_submissions_test.exs test/kyuubiki_web/api/solver_completion_receipt_api_test.exs
```

The tests reset shared test stores. Do not point them at installed runtime data
or research data that should be retained.

## Boundary At This Checkpoint

This is local solver completion-receipt evidence, not atomic result publication or numerical qualification.

Job state and result storage are still separate operations. Cancellation after
the runner's terminal check, or process/storage failure between result storage
and the completed-state write, is not fenced by this repair. A transactional or
recoverable publication contract is still required for those boundaries. The
tests must not be cited as exactly-once execution, cross-store crash recovery,
or qualification of every RPC consumer, physical operator, and distributed mode.
PostgreSQL and remote Rust Agent execution were not exercised in this run.

The [job snapshot report](job-snapshot-reliability-20260927.md) covers the earlier
atomic single-job updates; those do not by themselves make job-plus-result
publication atomic. No solver trust level or remote evidence grade is promoted.
