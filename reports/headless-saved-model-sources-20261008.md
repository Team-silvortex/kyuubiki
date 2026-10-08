# Headless Saved Model Sources

## Scope and Failure

October 8, 2026. macOS ARM64 source overlay on `fe0f025d` (Daji 3.5.2 commit
label), with workspace/package metadata still 3.5.0 and installed App/runtime
baseline still 3.4.0. No App rebuild, remote deployment, publication or version
change is performed by this follow-up.

An effective batch fingerprint binds saved-reference tokens, not the content
returned by a later library read. A mutable model can change while keeping the
same model ID. The native loader formerly retained no source-content identity
and ignored an `expected_model_source` supplied by a caller.

The new content-mismatch regression failed against that implementation: a
valid GET response followed by a mismatching pin still advanced toward submission,
producing a transport failure against the closed controlled responder instead
of a pre-submit contract refusal. The repaired test passes across mutable-model,
saved-version, registered-FEM and explicit direct-mesh routes.

## Repair

The native Rust execution SDK exports `HeadlessModelSource`, its source-kind
enum, schema constant and `headless_saved_model_source`. The common loader
captures the versioned SHA-256 of an explicit saved record's identities, kind,
complete payload, and present material/model-schema metadata. Model sources
also bind their latest-version label when present, without assigning that
version to the mutable-model job.

The implementation reuses the exact streaming JSON encoder. Its domain-separated
hash retains tiny and adjacent floating-point changes, sorts object keys and
preserves array order. It borrows payloads and uses a 16 KiB encoding buffer
instead of retaining another encoded model document. Existing batch fingerprint
encoding and legacy TaskIR/patch/lineage hashes are unchanged.

A caller pin must be supported, closed, correctly typed and valid. Wrong selected
ID/source type or malformed descriptors fail before GET. Valid descriptors whose
content or parents differ fail after the read and before artifact upload, solver
POST, job creation or subsequent steps. Pins are not solver input fields.

Successful saved-reference outcomes retain `model_source`; the combined action
retains matching outer and nested descriptors. Inline direct-mesh output uses
null rather than inventing a saved source. The registered output is available
to ordinary workflow bindings and survives report compaction.

Research evidence requires supported source descriptors for saved-reference
actions and checks reference identities, parent hints, optional pins, prior-output
bindings and combined-receipt agreement. It does not trust resolved-payload labels
as a substitute for the effective batch. Legacy run JSON is readable but missing
saved-source identity cannot qualify new evidence by backfilling a guessed hash.
KCore uses this same native evidence verifier.

See the [source contract and recovery guide](../docs/headless-saved-model-sources.md).

## Real Service Regression

`real_saved_source_pins_reject_mutable_drift_and_preserve_historical_version_results`
starts temporary source Orchestra and two Rust Agents with independently owned
state and cleanup guards. It creates a project and four-element axial bar through
the native SDK. Length is 1 m, area 0.01 square m, Young's modulus 210 GPa,
and initial tip force 1000 N.

The initial pinned mutable-model solve agrees with `F L / (E A)` within `1e-12`
relative error. Creating a new model version with force 2000 N updates the current
model and changes its source descriptor. Reusing the old pin refuses the first
step: no job, extra project, downstream execution or Agent calculation is added.

An explicitly selected new pin computes a new job whose displacement is twice
the initial value within `1e-12` relative error. A pinned solve/wait of the original
saved version still returns the original displacement and source, and its library
record is unchanged. The successful jobs and Agent calculation count are exactly
three. This is a bounded independent analytic check, not broad solver qualification.

The saved-version batch and report are written to temporary JSON, reloaded and
used to reproduce the same research evidence without another calculation.
Corrupting the retained source digest refuses evidence. Unit fixtures separately
exercise parent/type/ID/hash corruption, missing sources, disagreeing combined
receipts and source bindings whose retained outputs contradict payload labels.

## Verification

Selected full-suite regression passes 881 tests, with no failed or ignored tests
in those successful runs:

- Native Headless SDK: 508 unit and 25 integration tests.
- Protocol: 114 unit and seven integration tests.
- KCore: six unit and 12 integration tests.
- Selected CLI suites: 103 tests, including all 52 `headless_live` tests,
  three research-round tests, 16 posture boundaries and 12 material-report tests.
- Standalone Rust controller SDK: 106 integration tests.

This follow-up adds nine native SDK tests and one actual-service regression.
Initial red runs and repeated targeted checks are not counted again. Native SDK,
CLI, Protocol and KCore all-target Clippy pass with warnings denied.
The workflow qualification minimum is 508 core unit tests plus 16 CLI boundary
tests, totaling 524 under its existing largest-binary-per-suite counting rule.
The updated qualification gate passes all 524 counted tests and its retained
development report passes re-verification; these repeats are not added to 881.

Formatting and diff whitespace checks, API surface, topology, function matrix,
extension standard, documentation book/inventory, organization and version-line
audits pass with their applicable self-tests. All 224 exact version contracts
remain 3.5.0. Source files remain below 800 lines and documents below 2000; the
main SDK guide remains 1999 lines, with this contract in a focused companion.

The tensor accepts the scoped native source-contract evidence without promoting
operational, installed or numerical qualification. Its posture remains zero
structural gaps, four maturity gaps, 19 evidence-grade gaps, 14 P0 gaps and Daji
status blocked. Unrelated readiness gaps are not erased by this follow-up.

## Boundaries and Reproduction

The source descriptor proves SDK-fetched snapshot consistency, not signed or
authenticated provenance, normalized solver-request identity, engine/operator
artifact identity or general physical accuracy. An unpinned run records what was
read but does not prove the caller selected a fixed baseline before execution.
A caller can fabricate reports and descriptors; hashes do not stop that.

The standalone Rust controller SDK is regression-tested but does not gain this
native execution producer. Python/Elixir, installed GUI/runtime, remote Linux,
1M scale and deployment qualification are not inferred from these source tests.

```text
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk -p kyuubiki-protocol -p kyuubiki-kcore
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test headless_live --test headless_research_round --test headless_execution_posture --test headless_artifact_generation --test headless_output_paths --test headless_material_results --test material_report
cargo test --offline --manifest-path sdks/rust/Cargo.toml
cargo clippy --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk -p kyuubiki-protocol -p kyuubiki-kcore -p kyuubiki-cli --all-targets -- -D warnings
```

Actual-service tests need permission for temporary loopback listeners and local
fixture processes. They do not require external network access or deployment.
