# Headless Write Acknowledgements

Date: 2026-10-07. Base checkout: `76aaf10d`, preserving the preceding
native binding, risk, job-result and original-receipt repairs. A write can
take effect even if its acknowledgement is lost. The native Headless service
executor must not turn that uncertainty into retry permission or downstream
execution.

## Repairs

The five test-first acknowledgement cases initially had three failures:
lost/invalid write replies, complete HTTP 500 replies, and deadline/upload
reply loss. The final five cases pass. The public failure-receipt tests also
initially failed because native runtime categories and preflight stages were
absent from the schema; the schema's positive-only step index contradicted
the native preflight report's reserved zero index.

`request_json_with_timeout` now distinguishes pre-write connection failure
from failure once writing has been attempted. For methods other than GET,
HEAD and OPTIONS, write/read/protocol/JSON failure gets the typed native
`service_request_outcome_unknown` diagnostic. Streamed artifact uploads use
the same gate for header/body/flush/read failure, including early body end.
The marker is conservative from the write attempt, not proof that all bytes
were delivered. Connection retries remain before writing; requests are not
replayed after acknowledgement loss.

The shared response parser checks the status-line version/code, declared
Content-Length syntax, duplicates, contradictory length/transfer framing and
received byte length. Truncated content is rejected even when its surviving
JSON happens to parse. Empty successful responses require HTTP 204; empty
HTTP 200 and malformed JSON do not acknowledge a write. Existing chunk
decoding is reused, not newly qualified as a complete HTTP implementation.

A complete HTTP 5xx error on a write is also uncertain: the server might have
accepted work before returning an error. Its details remain available but
cannot grant replay. This precedes the old artifact-proxy diagnostic for
HTTP 500; HTTP 413 retains that original classification. Complete 4xx replies
retain their existing error handling. Read-only calls and connection failures
before writing retain their prior diagnostics.

`kyuubiki.headless.service_request_outcome_unknown` reports stage `transport`,
`retryable: false` and strategy `none`. The batch stops without bindings from
that step or later writes. Zero executed steps means zero validated successful
acknowledgements, not zero server-side effects. TaskIR keeps its specialized
`operator_task_outcome_unknown` classification and inspection rules.

The public failure-receipt v1 schema declares the native unknown-outcome,
job-receipt, result-unavailable and binding categories plus the native CLI
preflight stages. Its policy forbids retry permission for those five runtime
categories. Step zero is reserved for non-retryable `run_preflight` contract
failures; execution steps use positive indices. Four native contract tests
check emitted receipt fields/enums, zero-index policy, non-replay policy and
registered asynchronous solver previews. This is selected contract-facet
coverage, not an arbitrary JSON Schema validator.

Expanded CLI checks exposed two legitimate explicit-mock material-template
failures, for dielectric screening and composite thermo-electric panels.
Registered asynchronous `solve`/`material_solve` previews now supply their
deterministic job ID with `preview_only: true`, without a numerical `result`.
The existing template tests remain unchanged, and research posture still
rejects mock. A wait-recovery fixture now uses the public `solving` state,
not the nonexistent `running`; deadline and same-job recovery checks remain.

## Actual Service Chains

Each new live test owns an Orchestra, SQLite store, two actual Rust Agents and
temporary data. A bounded test relay forwards the original request, consumes
the actual complete 2xx response, and closes the caller connection without
delivering that response. No response is fabricated, no production fault
switch is added, and no existing installed service is restarted.

`real_project_commit_with_lost_ack_is_unknown_without_replay_or_later_writes`
creates a project through the native SDK, then performs a separate explicit
CLI experiment with a different project name. Both callers report unknown
outcome and zero successful steps. The backend contains exactly those two
committed projects, not the downstream project. The relay records exactly
two requests and two discarded replies with no errors. Job count and Agent
execution counts stay zero, and no TaskIR dispatch journal is created.

`real_job_acceptance_with_lost_ack_computes_once_per_explicit_request` submits
a bar solve through the SDK, loses its actual acknowledgement and stops the
batch. A separately authorized CLI experiment submits a different force and
also stops. The real backend has two distinct jobs, exactly two request/reply
pairs and exactly two Agent calculations; downstream project count stays zero.
Each completed tip displacement matches independent `FL/EA` within relative
`1e-12`. Actual CLI failures exit nonzero, retain identical stdout/report-file
reports and emit `headless_execution_failed` with `retryable: false` on JSON
stderr.

Only the test harness inspects the discarded reply to learn its job ID and
observe completion. This is a fault-test oracle, not a product recovery API or
a claim that the caller can discover a lost submission ID. The CLI experiment
is not a replay of the SDK's unknown request. The HTTP 500 case is a static
transport fixture, not a real committed-write-then-500 server fault experiment.

## Verification

Final macOS ARM64 source checks passed:

- Complete Headless SDK: 336 unit tests in 31.42 seconds and 25 integrations.
- Complete Protocol: 114 unit tests in 0.42 seconds and seven integrations.
- Complete `headless_live`: 30/30 passed in 25.98 seconds.
- Native CLI binaries: 182 and 16 unit tests. CLI surface, execution posture,
  parameter patch, research round, task completion and wait recovery:
  1, 16, 2, 2, 5 and 1 integration tests respectively, all passed.
- CLI/SDK/Protocol test-target Clippy passed with warnings denied; Rust
  formatting and whitespace checks passed. Cargo and Mix lockfiles unchanged.

Remote Ubuntu x86_64 tests use an unprivileged disposable container with
Rust 1.88.0, Elixir 1.19.5/OTP 28, 8 CPUs, 8 GiB memory and 512 PIDs, four
Rust test threads for CLI tests and two Erlang schedulers.

- Complete Headless SDK: 336 unit tests in 31.38 seconds and 25 integrations.
- Complete Protocol: 114 unit tests in 0.38 seconds and seven integrations.
- Complete `headless_live`: 30/30 passed in 41.35 seconds.
- Native CLI binaries: 182 and 16 unit tests; the same six additional CLI
  integration targets above passed without ignored or filtered tests.
- SHA-256 matched 20 selected final source/support/schema/manifest/lock files.
  Both Cargo and Mix lockfiles remained unchanged.

All completed targets above had zero ignored or filtered tests. The full live
suite includes existing fixture-based cases as well as real-Agent tests;
the two new actual-write cases have the concrete scope stated above.
Correctness timings are not throughput measurements.

The test container was removed, and its 514 MiB temporary source/build/test
directory was deleted with absence verified. Installed services, deployed data
and shared public dependency caches were not removed.

Runtime API contracts, module topology/matrix, tensor self-test and actual
tensor validation, documentation book/inventory and project organization audit
passed. Source/documentation limits remain 800/2000 lines with zero tracked
debt. Evidence is registered as `verified`, not installed or operational.
The tensor still reports four maturity gaps, 19 evidence-grade gaps and
14 P0 gaps; release status remains blocked.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Scope

This qualifies native source acknowledgement-loss handling, not installed
Apps, standalone Rust/Python/Elixir parity, authenticated deployment or full
HTTP framing/security. Chunk-trailer completeness, unsupported transfer
encodings and ordinary non-deadline response size/slow-read budgets remain
separate work. Parsed JSON still needs action-specific acknowledgement checks;
a syntactically valid object alone does not prove a valid semantic receipt.
Query-shaped POST calls intentionally receive conservative write policy.

There is no submission idempotency key, durable acknowledgement recovery,
exactly-once execution or atomic whole-batch guarantee. Real 2xx response loss
tests show no automatic replay in these chains, not global replay prevention
across independent clients or restarts. Unknown outcomes require explicit
inspection and reconciliation before continuation. No solver algorithm,
engine implementation, release metadata, commit or App bundle was changed.
