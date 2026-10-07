# Headless Job Read Gates

Date: 2026-10-07. Base checkout: `0712b88d`, daji 3.5.0 source overlay.
Earlier uncommitted changes and historical evidence remain preserved.

## Repairs

Native `job_wait` and `result_fetch` previously selected a trimmed ID from
canonical/camel aliases without checking contradictions. A malformed
`prefer_job_result` silently became true; its camel alias was ignored. Optional
project/version/case association hints did not constrain the returned job.
Result identity and completed status were checked, but optional association
metadata could disagree across job and result responses without rejection.

The shared read guard now requires exact usable task IDs and consistent aliases.
Optional project, saved-version and simulation-case hints must be safe strings;
saved-version null is permitted only as an explicit unversioned constraint.
Every polling reply checks supplied hints. Repeated top-level context must agree
with the job. On separate result reads, optional nested job/context must agree
with verified job metadata. Hints are optional; this is not full Job schema
validation or a requirement to manufacture metadata in the result endpoint.

Both result preference aliases must be boolean and agree; default true and
metadata-only reads for false remain unchanged. Malformed preferences fail before
any read and before combined saved-version solves submit computation. Those
combined actions retain the selected version and any explicit project hint
through waiting and fetching rather than only presenting a local output label.

The existing `contract_failure` category at `validation` uses the explicit
`job_read_invalid:` detail, nonretryable with no replay strategy. Invalid reads
publish no success bindings and stop later independent writes. Existing job-ID,
status, completion and object-result gates remain distinct. Neither classified
read failure nor zero successful combined steps proves that an earlier solve
did not occur; inspect and reconcile before an explicit read continuation.

## Tests

Seven new SDK tests cover:

- Unmodified task and optional parent/case IDs, unsafe characters, value types
  and conflicting canonical/camel aliases before network I/O.
- Boolean preference types and aliases before reads or combined submissions.
- Requested association mismatches, missing fields and contradictory repeats
  for waiting and both result preferences.
- A matching active poll followed by a completed poll for the wrong version.
- Cross-response optional job/context contradictions on separate result reads.
- Valid aliases, explicit null version constraints, full result preservation,
  no raw result mirror and correct embedded/separate read paths.
- Combined saved-version actions stopping at wrong version or explicit project
  context after submission, before result fetching or later writes.

Bounded socket fixtures and batch call logs separate request-option failures
from read failures. Existing healthy combined-solve fixtures now include their
real version association; the production guard is not weakened to accept missing
context. Empty IDs can also be rejected by batch preflight, independently of the
executor gate. Fixture route behavior is not solver or platform qualification.

The new live test owns temporary SQLite/Orchestra and two Rust Agents. A saved
axial bar is computed once through `solve_and_wait_from_model_version`. The
test-only bounded relay receives actual successful GET replies, removes exactly
the job's version ID, and returns complete HTTP 200 with correct byte length.
Original and altered replies are compared exactly. This fault is injected by
the test, not a backend fault observed in normal deployment.

`job_wait`, embedded `result_fetch`, metadata-first `result_fetch` and a separate
CLI read all reject the missing version association. CLI exits nonzero with
nonretryable JSON stderr and stdout/report parity. The four faulty reads create
no downstream project or job. The completed job/result remain unchanged. Both
normal result preference modes subsequently return the same numerical result;
the Agent execution count remains one. Displacement matches independent
`FL/EA` within relative `1e-12` for the 1000 N test load. Receiving a result object
in an embedded reply is not the same as accepting it after context validation.

## Verification

macOS ARM64 source regression:

- Headless SDK: 364 unit tests in 33.01 seconds plus 25 integrations, all passed.
- Protocol: 114 unit tests in 0.43 seconds plus seven integrations, all passed.
- Complete live suite: 36/36 in 24.85 seconds, all passed.
- CLI binaries: 182 and 16 unit tests; CLI surface, execution posture, parameter
  patch, research round, task completion and wait recovery: 1, 16, 2, 2, 5 and 1
  integrations respectively, all passed.
- CLI/SDK/Protocol test-target Clippy passes with warnings denied.

Full targets have zero ignored/filtered tests; focused development runs filter
unrelated tests. Initial test assertions for preflight-empty IDs and a route
spelling were corrected without relaxing production validation. Durations are
not throughput benchmarks. Local tests own and remove scratch resources.

Runtime API, module topology, matrix, extension standard, tensor self-test and
validation, documentation book/inventory, project organization and version-line
checks pass. The matrix has 13 modules and 11 paradigms; the tensor retains zero
structural gaps, four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps, with
Daji readiness blocked. Source/document limits remain 800/2000 with zero tracked
debt. Version audit checks 224 exact contracts with zero mismatches. Formatting
and whitespace checks pass; no evidence is promoted beyond native `verified`.

This follow-up stays local; no new remote source transfer or Linux verification
is claimed. Earlier Linux reports remain historical evidence. No installed
service, persistent development database or deployed data is modified.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

These are native request/read association checks, not authenticated ownership,
immutability, full JSON Schema, exact input/result digest binding, durable
checkpoints, scientific correctness, atomic batch, exactly-once or recovery
guarantees. Job metadata can be edited independently; a changed association needs
explicit caller reconciliation. Without a supplied hint, there is no caller
project/version/case requirement. Optional repeated result metadata is checked,
but the normal result endpoint provides only job ID and result; its numerical
payload provenance is not independently certified by this change.

Cross-response association faults are fixture-checked; the actual relay test
targets missing version association on the first job read. The selected version
constraint is retained in combined actions, not universally inferred for every
arbitrary solve/read batch. Explicit null version means a declared null field,
not missing metadata. Result preferences govern fetching, not `job_wait`.

Evidence is native source `verified`, not installed GUI, Linux/Windows,
Python/Elixir parity, generic mesh, sustained scale or research qualification.
No version bump, commit, push or APP build/install is performed.
