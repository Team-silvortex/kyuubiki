# Headless Submission Association Gates

Date: 2026-10-07. Base checkout: `0712b88d`, daji 3.5.0 source overlay.
Earlier uncommitted changes and historical evidence remain preserved.

## Repairs

The native submission receipt gate checked job ID/status but not the transmitted
project/version context. A complete successful reply could therefore acknowledge
another association. Saved-version actions could then present the requested
version as a local label even though the submitted job referenced another one.

Native FEM, composite panel and explicit mesh submission now retain only the two
supported canonical association fields and compare them against the nested job
before publishing bindings or interpreting terminal status. Repeated top-level
association fields must agree with the job, even without request constraints.
Missing, wrong or malformed requested context retains `job_receipt_invalid:`
under `service_request_outcome_unknown`, nonretryable with no replay strategy.
The job may already exist or have calculated; a failed combined step is not
proof of nonexecution. Matching failed/cancelled receipts remain known failures.

Explicit IDs must be usable strings before submission. Native FEM preserves
inline model association through artifact transport; outer request context
retains precedence. The gate copies only project/version fields, not another
large mesh. Saved references retain their verified parents from the library read.

Orchestra's shared context derivation previously used checkpoint ownership even
when the caller explicitly named a different project. That was an existing,
tested precedence policy, not an accidental untested branch. It now deliberately
rejects contradictory explicit pairs with HTTP 422 and
`:model_version_project_mismatch` before job creation/dispatch. Version-only,
matching pairs and project-only submissions remain supported. No numerical
algorithms or Engine/Agent execution rules change.

## Tests

Nine new SDK tests cover all 49 native direct solver routes with missing requested
association; corrupt values and terminal statuses; contradictory repeated
fields; composite, explicit mesh and native mesh routes; saved model/version
native and fallback routes; matching optional/inline context; and invalid
project options before I/O. Bounded socket fixtures and two-step batch call logs
verify that invalid replies produce no successful bindings, polls, result reads
or later independent writes. Existing healthy artifact and saved-version
fixtures now supply the association their requests actually send.

The Elixir regression checks JSON catalog, raw graph and direct truss requests
plus atom-key callers. Contradictory pairs leave the job list and both project
bundles unchanged. Version-only and matching requests keep their real lineage.
Both old precedence expectations were replaced with the new defined policy.

The new live test owns temporary SQLite/Orchestra and two actual Rust Agents.
A contradictory axial-bar request first returns 422 with no job or calculation.
Two separate SDK/CLI experiments then submit different loads with a matching
saved-version context. A bounded test relay receives each actual successful POST
reply and removes exactly its project ID or version ID, preserving complete HTTP
framing. Original and altered JSON are compared exactly; reply mutation is a
test fault, not a fault observed during ordinary deployment.

Both callers stop without replay or downstream project creation. CLI exits
nonzero with nonretryable JSON stderr and stdout/report parity. The discarded
reply is an independent test oracle, not a recovery API exposed to users. An
unproxied test read verifies each accepted job/result and project/version
association. Agent counters show two distinct jobs and two calculations, one per
explicit experiment. Displacement matches independent `FL/EA` at relative
`1e-12` for 1000 N and 2000 N loads; the saved version remains unchanged.

## Verification

macOS ARM64 source regression:

- Headless SDK: 373 unit tests in 33.30 seconds plus 25 integrations, all passed.
- Protocol: 114 unit tests plus seven integrations, all passed.
- Complete live suite: 37/37 in 40.91 seconds, all passed.
- CLI binaries: 182 and 16 unit tests; selected CLI integration targets: surface
  1, execution posture 16, parameter patch 2, research round 2, task completion 5,
  wait recovery 1, all passed.
- Orchestra: 1275 tests, zero failures, 30 skipped, in 27.4 seconds. The skipped
  environment-dependent cases are not counted as executed qualification.
- CLI/SDK/Protocol test-target Clippy passes with warnings denied.

Rust full targets have no ignored/filtered tests. Focused development runs filter
unrelated tests. Initial fixtures were corrected to use the existing endpoint
spelling, composite required fields and saved-version batch endpoint options;
production validation was not weakened. The first full Elixir run identified a
second test expecting the old precedence policy; the final full run passes.
Durations are regression timing, not throughput benchmarks.

Runtime API, topology, matrix, extension, tensor self-test/validation, book and
inventory, organization and version checks pass. Source/document limits remain
800/2000. The tensor now includes the previously written library receipt,
saved-reference and job-read evidence shards, alongside this follow-up, so they
are actually loaded and validated. All new evidence stays native `verified`;
release qualification gaps are not closed by this change.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

From `apps/web`:

```text
MIX_ENV=test ERL_FLAGS='+S 2:2' mix test --seed 0
```

## Limits

These checks establish request/receipt association consistency, not access
authorization, authenticated ownership, an immutable input digest, full schema,
result provenance, scientific qualification, atomic batches, durable checkpoints
or exactly-once execution. If an acknowledgement is unknown, inspect and
reconcile actual control-plane records before authorizing any continuation.
Artifact upload can precede a backend context rejection; zero created jobs is
not a guarantee of zero earlier uploaded artifacts.

Without a transmitted field there is no corresponding caller constraint. Native
catalog/graph SDK submissions still send graph/ID and input artifacts only; this
change does not forward their unused project/version payload options. Their
repeated reply fields are checked, and direct HTTP backend workflow clients use
the stricter shared context rule. Simulation-case hints are read constraints,
not accepted FEM submission context in this change.

Explicit mesh submission remains fixture-tested rather than deployed endpoint
qualification. The live numerical oracle covers a simple bar, not all 49 solver
algorithms. No new remote transfer, Linux/Windows, installed App or cross-language
acceptance is claimed. Local scratch resources are removed by owned test guards.
No production services, deployment credentials or research data are changed.
No version bump, commit, push or App build/install is performed.
