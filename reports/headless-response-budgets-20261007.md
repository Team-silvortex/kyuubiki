# Headless Native HTTP Response Budgets

Date: 2026-10-07. Base checkout: `0712b88d`, daji 3.5.0 source overlay.
Existing uncommitted changes and historical evidence are preserved.

## Repair

Ordinary native service requests previously used `read_to_string` until EOF,
without a response byte cap or an absolute network deadline. A peer could keep
the socket alive with slow writes, or exhaust memory before receipt validation.
Artifact upload acknowledgements had a separate unbounded reader. Deadline
requests were already bounded, but that did not protect these ordinary paths.

Both now use the same deadline reader. Default ordinary network time is 30 s;
single job detail/result/model/version reads have a 600 s total budget and retain
the 30 s I/O inactivity ceiling. Explicit wait, TaskIR and original-inspection
deadlines retain precedence. Artifact upload has one 600 s network budget across
DNS, connection, header/body writes and acknowledgement reads, using a fixed
64-KiB file buffer. Received or sent bytes do not renew total time. Every native
HTTP connection path now uses the existing bounded DNS resolver; at most four
noncancellable DNS lookups may remain outstanding per process.

Read-only endpoint policies bound wire bytes, including headers and chunk
framing: ordinary service/TaskIR responses 64 MiB; job status and artifact upload
receipt 8,000,000 bytes; single job detail, result, model or version 512 MiB;
original dispatch-result receipt 10 MiB. Query strings do not change the selected
policy. The large response cap is independent of the configurable upload cap.
These constants are documented in the SDK reference, not hidden solver knobs.

HTTP headers have a separate 64-KiB cap including their delimiter. Header
validation shares the existing duplicate/invalid-length and contradictory-framing
guard with the response parser. An advertised length beyond the remaining wire
budget is rejected without reserving its requested memory or waiting for a body.
Unframed/chunked reads also enforce observed byte count. Header delimiter scans
overlap only the last three bytes and stop after finding the header; large bodies
are not repeatedly rescanned. Buffer growth is bounded and uses fallible exact
reservation. Non-chunked decoding borrows the original body rather than copying
an entire second String; chunked decoding remains owned.

Read size/allocation failures have a public nonretryable
`service_response_limit_exceeded` category at `transport`, with retry strategy
`none`. The v1 failure schema declares and constrains this additive category.
Post-write faults take precedence: an oversized acknowledgement is still
`service_request_outcome_unknown`, and TaskIR keeps its specialized unknown
outcome. Complete HTTP rejections keep their previous diagnostics when they fit
the budget. No request replay is added and failed reads publish no success
bindings or downstream actions. Client rejection does not roll back a committed
write, stop server-side computation or invalidate the retained original result.

The change adds no alternate engine, numerical implementation or dependency.
Numerical ownership remains in Solver; Engine/Agent retain execution authority.

## Tests

Fourteen new SDK tests cover route and total-time policies, split delimiters,
exact wire boundaries, terminated/unterminated oversized headers, invalid and
duplicate lengths, framing conflicts, advertised oversize, observed unframed/
chunked growth, slow trickle under ordinary GET/POST deadlines, read/write error
classification, no successful bindings or later writes, exact streamed file
bytes, malformed/truncated JSON acknowledgements and body borrowing by pointer.
Ordinary job/result reads actually receive values exceeding 8,000,000 bytes and
retain the entire value; this is not only a constant/policy assertion. Existing
TaskIR large-result and original-receipt limit tests remain in the full suite.
The public failure-schema regression includes the new category and no-retry rule.

Two new real service tests own temporary Orchestra/SQLite and two Rust Agents:

- Actual SDK and distinct CLI project writes commit upstream. A bounded test
  relay then replaces their HTTP reply with an oversized length declaration.
  Both clients report unknown writes and stop before downstream actions. Exactly
  the two explicitly requested projects exist, with their actual receipt IDs;
  no jobs or Agent computations are created. CLI JSON stdout/report parity and
  nonretryable stderr are checked by the shared live helper.
- An actual bar job computes once. Its completed detail reply is captured before
  the same deliberately oversized declaration is sent to SDK and CLI readers.
  Both reject it as a nonretryable resource fault without successful result/job
  bindings or a later project. CLI stdout/report parity and error policy hold.
  Unproxied result reading succeeds, exact stored job/result remain unchanged,
  one job and no projects exist, and Agent execution count remains one. The
  public tip displacement agrees with independent `FL/EA` at relative `1e-12`
  for a 1000 N load. This oracle does not establish general solver qualification.

The injected length is not a real 512-MiB backend result or an observed deployment
fault. The relay captures the actual small upstream acknowledgement first; it
does not invent committed database records or calculation outcomes. Tests have
bounded connections, frames, timeouts and owned temporary-path cleanup; no
production fault injection switch is introduced. An initial live assertion read
a nonexistent displacement-array field; it was corrected to the existing public
`tip_displacement` field without changing any solver/result output.

## Verification

macOS ARM64 source regression:

- Headless SDK: 394 unit tests in 33.43 s plus 25 integrations, all passed.
- Protocol: 114 unit tests plus seven integrations, all passed.
- Complete real-service suite: 41/41 in 25.89 s, all passed.
- CLI binaries: 182 and 16 unit tests; selected integrations: surface 1,
  execution posture 16, parameter patch 2, research round 2, task completion 5,
  wait recovery 1, all passed.
- CLI/SDK/Protocol test-target Clippy passes with warnings denied; Rust formatting
  and diff whitespace checks pass.

Full selected targets have zero ignored/filtered tests. Focused runs filter
unrelated tests. Durations describe test execution, not throughput or RSS
benchmark measurements. The backend is exercised by the real-service suite;
no new full Elixir suite or Linux test run is claimed in this native follow-up.

Runtime API, topology, matrix, extension standard, tensor self-test/validation,
documentation book/inventory, organization and version-line checks pass. The
tensor loads the new native `verified` evidence without qualification promotion:
zero structural gaps, four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps;
Daji readiness remains blocked. Organization limits remain 800 source/2000 doc
lines with zero tracked debt; all 224 exact version contracts match 3.5.0.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

The wire cap does not bound total process RSS, parsed-value size, concurrent
requests, large diagnostics, full retained step bindings or `job_fetch` raw/
result mirrors. Fallible buffer reservation cannot prevent all OS OOM behavior
or allocation failures in other layers. No full result streaming-to-disk or
process-wide admission policy is introduced. The 512-MiB cap intentionally
rejects larger replies; raising model upload limits does not raise this cap.

Responses still require EOF. A peer holding a complete framed body open is
bounded by the deadline, not accepted early. This is not a new complete HTTP
parser, TLS implementation or framing/fuzzer qualification. Local file access,
JSON serialization/parsing, scheduling delays and the OS are not preemptible by
the network deadline. Existing request payload and artifact integrity contracts
remain separate; ordinary defaults and wire limits are fixed native policies in
this change, while explicit TaskIR/wait/inspection time budgets retain precedence.

Scope is native Rust `ServiceHeadlessExecutor` and its reference CLI. Independent
clients in `sdks/`, installed Apps, authenticated/durable deployments, 1M-scale
performance, exactly-once guarantees and scientific result provenance are not
newly qualified. The installed local baseline remains 3.4.0; no build/install,
version bump, commit or push was requested. Historical evidence is not promoted.
