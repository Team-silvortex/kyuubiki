# Headless Submission Receipts

Date: 2026-10-07. Base checkout: `0712b88d` with the current 3.5.0 metadata
overlay and this native submission-receipt repair. Prior reports keep their
original source fingerprints. A syntactically valid HTTP success response
does not establish a usable job submission acknowledgement.

## Repair

The new seven-case test module initially had six failures: invalid replies
still reached later independent writes, and explicit failed/cancelled jobs
were treated as acknowledged success. All seven cases now pass. Two fixture
inputs were corrected to satisfy existing composite and mesh request contracts;
the production gates were not relaxed to accommodate them.

`normalize_job_submission_result` is now fallible. It requires an explicit job
object, a string job ID accepted by the existing safe path-segment policy,
and one of the eight public statuses. Whitespace and unsafe IDs are rejected
as received, not trimmed into validity. Optional top-level ID/status must match
the nested fields. Rejected field values are not echoed in validation errors.
Progress remains optional; this is selected receipt-facet validation rather
than a complete job-schema implementation.

Semantic failure after a POST uses the existing `service_request_outcome_unknown`
receipt, retaining a `job_receipt_invalid` detail. Retry permission remains false,
strategy is none, no bindings are published from that step, and later batch
steps are not called. Zero successful steps is not proof of zero backend effects.
Valid failed/cancelled jobs stop through the existing known terminal-job failure
path instead of acquiring the unknown-write marker. Active receipts acknowledge
submission only; a completed receipt without a result is still not numerical
validation. Existing job-wait/result-fetch gates remain separate.

Final review reproduced a related classification bug: a valid failed receipt
whose reason said `failed to connect` acquired `transport_failure` and retry
permission. Explicit `job_terminal_failed`/`job_terminal_cancelled` markers now
precede free-text transport, queue, timeout and authorization heuristics.
The terminal case checks both submission and job-wait with eight reason strings
for each of two terminal states: 32 combinations, all stopped, known and
non-retryable. The original reason remains available for inspection. No new
public failure category or solver policy is introduced.

The shared gate covers all 49 registered direct FEM routes, the composite
thermo-electric panel, both workflow submission routes, explicit direct mesh
and native mesh routing, plus model-version solve/solve-and-wait. Version-chain
failure is checked before any job polling or result read, including the explicit
mesh route using a caller-provided supported study kind. Solver algorithms,
Engine/Agent dispatch contracts and existing risk authorization are unchanged.

The malformed HTTP fixtures cover absent/non-object job envelopes, missing and
wrongly typed IDs, whitespace, path/control characters, unsupported statuses
and conflicting repeated fields. Six valid active/completed states retain job
bindings without fabricating a numerical result; failed and cancelled states
remain known failures. Empty request models in route-matrix fixtures exercise
client acknowledgement handling, not physical solver correctness.

## Actual Service Chain

The new live test owns temporary SQLite data, an Orchestra and two actual Rust
Agents. Its bounded test relay forwards each original POST, receives the actual
complete successful upstream reply, removes only nested `job_id`, and sends a
new complete HTTP 200 response with corrected byte length. This is explicit
test-only response corruption, not evidence that the backend emitted an invalid
reply in normal deployment. Existing loss-only relay modes remain unchanged.

The SDK submission at 1000 N and a separate explicit CLI submission at 2000 N
both stop with unknown-outcome receipts and zero successful steps. The relay
records exactly two requests, two original replies and two deliberately
corrupted replies, with no errors. Comparing original and corrupted JSON proves
that only the job ID was removed. The backend holds two distinct jobs; the
Agents calculate exactly twice in total and no downstream project is created.
Both displacements match independent `FL/EA` within relative `1e-12`.

Only the test harness reads the captured original IDs to observe the existing
calculations. This is a test oracle, not a product recovery API. The distinct
CLI experiment is not a replay of the SDK's unknown request. The CLI exits
nonzero, emits non-retryable JSON failure stderr, and retains equal stdout and
report-file JSON. The full live suite includes fixture-based cases as well as
actual-Agent chains; these scopes must not be counted as all real solves.

## Verification

Completed macOS ARM64 source checks:

- Headless SDK: 343 unit tests in 32.33 seconds plus 25 integrations.
- Protocol: 114 unit tests in 0.56 seconds plus seven integrations.
- Complete live suite: 31/31 in 23.40 seconds.
- Native CLI binaries: 182 and 16 unit tests. CLI surface, execution posture,
  parameter patch, research round, task completion and wait recovery: 1, 16,
  2, 2, 5 and 1 integration tests respectively, all passed.
- CLI/SDK/Protocol test-target Clippy passed with warnings denied.

Completed remote Ubuntu x86_64 SDK/Protocol checks in an unprivileged temporary
container with Rust 1.88.0, Elixir 1.19.5/OTP 28, 8 CPUs, 8 GiB memory and 512 PIDs:

- Headless SDK: 343 unit tests in 32.21 seconds plus 25 integrations.
- Protocol: 114 unit tests in 0.38 seconds plus seven integrations.
- Complete live suite: 31/31 in 42.43 seconds.
- Native CLI binaries: 182 and 16 unit tests; the same six additional CLI
  integration targets above passed without ignored or filtered tests.
- CLI/SDK/Protocol test-target Clippy also passed with warnings denied on Linux.
- SHA-256 matches 24 selected final source, schema, manifest, configuration-source
  and lock files. Both Cargo and Mix lockfiles remain unchanged during tests.

All completed full targets above have zero ignored or filtered tests. Timings
are correctness-run durations, not throughput benchmarks. The temporary test
workspace contains only selected public source, schemas and dependency source,
not private environment files or deployed configuration. Existing installed
services and data are not restarted, replaced or copied into this experiment.
The final full macOS and Linux runs include the terminal-marker follow-up;
intermediate passing results are not used for that qualification. A final
equivalent ASCII-escape spelling of the Unicode-ID fixture was separately
rechecked through all seven submission cases on both hosts (0.15/0.11 seconds);
those focused runs filter unrelated tests. Production code is unchanged by
that spelling adjustment. Rust formatting and whitespace checks pass.

Runtime API, topology, matrix, extension standard, tensor self-test/validation,
documentation book/inventory, version-line and project-organization checks pass.
Source/documentation limits remain 800/2000 lines with zero tracked debt.
Both owned temporary server workspaces, approximately 562 MiB each, were deleted
with absence verified after their containers were removed. Shared public
dependency caches, installed services and deployed data remain intact.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

This qualifies native source submission acknowledgement handling, not installed
Apps, standalone Rust/Python/Elixir parity, authenticated deployment, long-run
scale, scientific validation or full HTTP parser security. Model/project write
acknowledgements and standalone job-fetch metadata semantics are separate work.
Consistent server-assigned ID fields do not prove input identity without a
stronger request-bound backend contract. There is no idempotency key, durable
receipt recovery, atomic batch, exactly-once guarantee or automatic discovery
of a lost job ID. Engine/Solver separation and all numerical gates are unchanged.

The tensor records this as `verified`, not installed or operational. Existing
four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps remain open; the
release gate remains blocked. No commit, push, version increment, installed
service restart or App rebuild is part of this repair.
