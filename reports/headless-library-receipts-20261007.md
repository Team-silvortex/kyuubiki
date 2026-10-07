# Headless Library Receipts

Date: 2026-10-07. Base checkout: `0712b88d`, current daji 3.5.0 source overlay.
This follow-up preserves preceding uncommitted repairs and version alignment.
Historical reports retain their original scope and fingerprints.

## Repairs

Five native actions previously accepted any parsed successful JSON as a library
write acknowledgement: project creation, update and deletion, model creation,
and version creation. Missing records or identities could count as successful
execution and allow subsequent independent writes. Normalization now validates
selected identity facets before returning `executed` or publishing bindings:

- Explicit project/model/version object and usable canonical identifier.
- Unmodified safe service-path IDs, with no trimming or repaired identities.
- Optional top-level identifiers consistent with the nested record.
- Project update/delete identity matching the requested project.
- Model creation parent matching the requested project, plus a usable latest
  version ID, which is a published output of the existing action contract.
- Version creation parent matching the requested model, a usable project ID,
  and consistent optional nested/top-level `model_version_id` aliases.

Semantic rejection after POST/PATCH/DELETE retains `library_receipt_invalid`
under the existing `service_request_outcome_unknown` category. Retry is false,
strategy is none, successful-step count is zero, and downstream independent
steps are not called. Rejected identifier values are not echoed in these error
details. Valid records retain the prior flattened fields, nested envelope and
raw response. This is not complete JSON Schema or payload validation.

The current library DELETE route returns HTTP 200 with the deleted project.
An empty 204 cannot provide that action's required project receipt; it therefore
fails the semantic gate. Generic transport acknowledgement handling still
accepts a valid 204. Complete 4xx diagnostics and pre-send failures are unchanged.

The actual happy-path chain also exposed a separate provenance bug: a saved
version was loaded and computed correctly, but its identifier was omitted from
the native FEM submission. Only the SDK output label referred to the version;
the server-side job had no version association. Registered native FEM version
solves now transmit the canonical reference. Explicit direct FEM project/version
context is preserved in inline and artifact-reference bodies; invalid explicit
context is rejected before connection or artifact upload. Orchestra retains
ownership of persisted associations; Engine/Solver algorithms are unchanged.

## Tests

The six library boundary tests cover every action, absent/non-object envelopes,
missing IDs, null/numeric/boolean IDs, blank/whitespace IDs, traversal/control/
encoded-slash/Unicode inputs, contradictory repeats, wrong requested objects
or parents, alias conflicts, valid output preservation and no-content deletion.
Each invalid fixture runs a batch with an independent later project write;
only the first SDK action is called, with no published ID binding.

Three additional context tests cover all 49 registered direct solver routes,
invalid explicit context before any network attempt or upload, and saved-version
reference forwarding. The existing large-model transport test now checks that
context remains in the small solve request, not the uploaded numerical model.
Route-matrix fixtures exercise client behavior, not all 49 physical solvers.

The new actual service tests own temporary SQLite and Orchestra resources.
The test-only bounded relay forwards original requests, receives actual
successful upstream replies, removes exactly one record identifier, and returns
a complete replacement HTTP 200 with correct byte length. This is an explicit
test fault, not a backend fault observed in normal deployment.

Corrupted creation, update, model creation, version creation and deletion all
stop the native SDK batch. Separate named project/model creation experiments
also check CLI nonzero exit, non-retryable JSON stderr and stdout/report parity.
Exactly seven corrupted writes occur: two project creates, one update, two model
creates, one version create and one deletion. Independent database reads prove
the committed records exist, the deleted project is absent, and no later project
or job was created. Captured original identities are test oracles only, not
product recovery APIs or authorization to replay an unknown write.

The normal chain uses two actual Rust Agents:

```text
project_create -> project_update -> model_create -> model_version_create
-> solve_and_wait_from_model_version -> result_fetch -> project_delete
```

All seven steps complete, the persisted job references the original project and
the new version, and the Agents execute one calculation. The initial model uses
1000 N; the new version uses 2000 N. The retrieved displacement matches independent
`FL/EA` within relative `1e-12`, proving this small example computed the new load,
not the initial version's load. The project is then explicitly removed. This
small analytical check does not establish general scientific qualification.

## Verification

Final macOS ARM64 source checks:

- Headless SDK: 352 unit tests in 32.65 seconds plus 25 integrations.
- Protocol: 114 unit tests in 0.45 seconds plus seven integrations.
- Complete live suite: 33/33 in 20.97 seconds.
- CLI binaries: 182 and 16 unit tests. CLI surface, execution posture, parameter
  patch, research round, task completion and wait recovery: 1, 16, 2, 2, 5 and 1
  integration tests respectively, all passed.
- CLI/SDK/Protocol test-target Clippy passed with warnings denied.

Full targets have zero ignored or filtered tests. Focused development runs
filtered unrelated tests; initial live fixture failures were corrected without
weakening the production gate. Run durations are not throughput benchmarks.

Remote Ubuntu x86_64 source checks in a non-admin temporary container with
Rust 1.88.0, 8 CPUs, 8 GiB memory and 512 PIDs:

- Headless SDK: 352 unit tests in 32.59 seconds plus 25 integrations.
- Protocol: 114 unit tests in 0.47 seconds plus seven integrations.
- Complete live suite: 33/33 in 49.19 seconds.
- CLI binaries: 182 and 16 unit tests; the same six additional integration
  targets above passed. Full targets have zero ignored or filtered tests.
- CLI/SDK/Protocol test-target Clippy passed with warnings denied on Linux.
- SHA-256 matches 24 selected source, schema, manifest, configuration-source
  and lock files on both hosts after the full tests. Cargo and Mix lockfiles
  remain unchanged. No private environment or deployed configuration is copied.

Local runtime API, topology, matrix, extension, tensor self-test/validation,
documentation book/inventory, organization and version-line checks pass.
Source/documentation limits remain 800/2000 lines with zero tracked debt.
Version audit checks 224 exact contracts with zero mismatches. Tensor checks
retain four maturity, 19 evidence-grade and 14 P0 gaps; readiness stays blocked.
Formatting and whitespace checks pass. The owned temporary server workspace,
approximately 562 MiB, was deleted with absence verified after its test container
was automatically removed. Shared caches, installed services and deployed data
remain intact. Local test suites also remove their owned scratch resources.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

Verified native source behavior is not installed APP acceptance, Windows,
Python/Elixir parity, authenticated deployment, scale or long-run qualification.
Create acknowledgements do not prove request-bound input identity; safe IDs and
parent consistency do not establish exact payload, current latest-version
membership, numerical correctness or complete persisted provenance. General
mesh routing and model-reference read semantics remain separately scoped.
This repair adds no idempotency key, durable unknown-ID recovery, automatic
retry, atomic batch or exactly-once guarantee. Neither server-assigned IDs nor
the compact result preview are resumable research checkpoints.

The coverage tensor records only `verified` native identity/recovery scope.
Existing maturity, evidence-grade and P0 release gaps remain open. No version
bump, commit, push, installed service restart or APP rebuild is performed.
