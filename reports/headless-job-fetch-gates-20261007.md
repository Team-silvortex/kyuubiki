# Headless Single Job Observation Gates

Date: 2026-10-07. Base checkout: `0712b88d`, daji 3.5.0 source overlay.
Earlier uncommitted changes and historical evidence remain preserved.

## Repair

Native `job_fetch` previously selected a trimmed request ID and normalized any
parsed response as a successful read. It did not verify the returned job
identity, public status, repeated fields or caller-supplied project/version/case
association. This remained an unguarded path alongside the stricter waiting and
result-reading paths.

The action now reuses the existing `JobReadRequest` and `validate_job_receipt`
guards: exact usable IDs, consistent canonical/camel executor aliases, an
explicit matching job object/public status, and optional caller association.
Repeated identity/status/context must agree. Explicit null version requires a
declared null, not absent metadata. Batch canonical required fields remain
unchanged; executor aliases do not extend batch preflight contracts.

Malformed identity/state replies fail as `job_receipt_invalid` at
`job_observation`; malformed options or association fail as `job_read_invalid`
under `contract_failure` at `validation`. Both are nonretryable with no replay
strategy and stop before successful bindings or later actions. A GET receipt
fault is not an uncertain write. Read transport failures and complete HTTP
rejections retain their existing diagnostics/policy.

All eight public states remain inspectable, including failed and cancelled
records. No terminal-failure execution rejection is applied to observation.
Successful read execution means only that the query succeeded. The detail
endpoint, reasons, nested job and existing raw/retained-result values stay
available for diagnosis; this action does not wait for computation or admit a
completed scientific result. The binding resolver does not prohibit arbitrary
diagnostic values from being forwarded: research callers must not substitute
them for the completed-result gate, which itself is not scientific validation.

The change adds no alternate Engine, numerical algorithm, backend state mutation
or new runtime dependency. It closes the query path by reusing shared guards
rather than adding another independent validation implementation.

## Tests

Seven new SDK tests cover malformed explicit job objects, missing/wrong/type-
invalid/padded identities, unsupported status, contradictory repeats, optional
project/version/case hints and aliases, explicit null versions, and normal
diagnostic observation of every public state. A valid failed-job query can
continue an explicitly chosen diagnostic chain; query success is distinct from
successful computation. HTTP 404 keeps its existing error handling.

The prior exact read-option matrix now includes `job_fetch`: malformed IDs,
unsafe characters, wrong value types and conflicting aliases fail before I/O.
Bounded socket fixtures and batch call logs check invalid reads stop before a
later independent project write and publish no job/result/raw success binding.

Two new real service tests own temporary SQLite/Orchestra and two Rust Agents:

- A saved bar computes once. The bounded test relay removes exactly one of job
  ID, status, project, version or case from actual completed detail replies.
  Original and corrupted JSON are compared exactly with valid HTTP framing.
  All five SDK reads and two CLI reads fail with their correct classifications;
  CLI exits nonzero with nonretryable JSON stderr and stdout/report parity.
  Unproxied SDK/CLI observation then succeeds. Stored job/result remain unchanged,
  no downstream project is created, and Agent execution count remains one.
  Displacement matches independent `FL/EA` at relative `1e-12` for a 1000 N load.
- Four opt-in records are seeded through the actual transactional store with
  queued/solving/failed/cancelled states and retained diagnostic result objects.
  SDK and CLI successfully observe each, while `result_fetch` rejects each as
  unavailable completed computation. Projects/jobs/results remain unchanged and
  both Agent execution counters remain zero. These seeded lifecycle records are
  not results of a numerical failure experiment or simulated scientific proof.

Reply corruption is explicitly test-injected, not a fault observed in ordinary
deployment. The relay has owned paths, bounded connections/size/timeouts and
automatic cleanup; no production fault switch is introduced.

## Verification

macOS ARM64 source regression:

- Headless SDK: 380 unit tests in 33.39 seconds plus 25 integrations, all passed.
- Protocol: 114 unit tests plus seven integrations, all passed.
- Complete live suite: 39/39 in 20.39 seconds, all passed.
- CLI binaries: 182 and 16 unit tests; selected CLI integrations: surface 1,
  execution posture 16, parameter patch 2, research round 2, task completion 5,
  wait recovery 1, all passed.
- CLI/SDK/Protocol test-target Clippy passes with warnings denied.

Rust full targets have zero ignored/filtered tests. Focused runs filter unrelated
tests. Durations describe regression execution, not throughput improvements.
Real Orchestra source is exercised by the complete live suite; no separate new
full Elixir test run is claimed because backend source was not changed in this
follow-up. The prior submission-association report retains its full Elixir run.

Runtime API, topology, matrix, extension standard, tensor self-test/validation,
documentation book/inventory, organization and version-line checks pass. Source
and document limits remain 800/2000 with zero tracked debt. New evidence is loaded
by the tensor index at native `verified` grade. The tensor retains zero structural
gaps, four maturity gaps, 19 evidence-grade gaps and 14 P0 gaps; Daji readiness
remains blocked. No qualification gap is closed by this local query repair.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

This is observation identity/state/association validation, not full Job schema,
authorization, immutable model/input provenance, validated numerical results,
scientific qualification, durable recovery or exactly-once execution. No hint
means no corresponding caller constraint. Valid failed/cancelled observation
does not automatically stop a chosen diagnostic chain; callers must separately
admit completed computation before using a result for research decisions.

The existing detail route can return large retained results; this change does
not make `job_fetch` a metadata-only performance optimization or eliminate raw
result duplication. Ordinary non-deadline GET reads still buffer the response
before semantic validation; this follow-up adds no response-allocation bound.
That transport/resource boundary needs separate hardening. Report compaction
does not bound the full response retained for execution bindings.
No new remote transfer, Linux/Windows, Python/Elixir parity, installed App or
sustained-scale qualification is claimed. Temporary resources are owned and
removed. No production data, credentials, version, commit, push or App
build/install is changed by this follow-up.
