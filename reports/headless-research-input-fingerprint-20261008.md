# Headless Research Input Fingerprint

## Scope

October 8, 2026. macOS ARM64 source overlay on `fe0f025d` (`daji 3.5.2` commit
label). Workspace/package metadata remains 3.5.0; the installed desktop/runtime
baseline remains 3.4.0. This work does not rebuild or install an App, publish an
SDK, change an engine/solver protocol, or claim remote acceptance.

The native Rust execution SDK and CLI share an effective-input identity gate
for research evidence. KCore research semantic export/verification uses the same
verifier. The standalone Rust controller SDK is regression-tested, but does not
gain the new execution producer/API; Python/Elixir parity is not claimed.

## Reproduced Failure

An old service report for `research_input = 10.0` was accepted as qualified
evidence for `research_input = 12.0`. Workflow ID, action/risk/index/completion
and generic validation summary all matched. The builder hashed the newly
supplied batch while reading the metric from the old report, without checking
that the report came from that effective batch.

The new `rejects_a_stale_report_for_changed_inputs_with_identical_validation`
test failed against the previous implementation: evidence was returned with
`qualified: true` and the old metric. The same test passes after the repair.

Legacy `headless_batch_content_sha256` also uses fixed 15-decimal number
normalization. For example, `1e-30` and `2e-30` collapse to the same digest.
It cannot serve as the lossless input admission gate.

## Repair

New native reports carry optional `execution_input`, with schema
`kyuubiki.headless-execution-input/v1` and SHA-256. Producers compute it from the
effective batch before binding resolution, dispatch, or report compaction.
Invalid batch/preflight reports omit it. Dry/blocked/failed reports may include
a captured fingerprint, but still fail existing research admission checks.

The exact v1 encoding and recovery rules are described in
[Headless Research Input Identity](../docs/headless-research-input-identity.md).
All batch fields except diagnostic warnings are bound. Sorted-key compact JSON
uses lossless numeric serialization, rather than legacy decimal rounding.
The schema string and NUL separator domain-separate the digest.

The implementation streams borrowed payloads into SHA-256 through a 16 KiB
buffer. It neither clones a full batch `Value` nor retains an encoded JSON
document. Unsorted object keys use temporary sorted references. Every value is
still traversed; no constant-time or zero-cost fingerprint claim is made.

Evidence creation and re-verification require exact fingerprint agreement.
Missing, unsupported, malformed or mismatched receipts reject admission.
Old run files remain readable, but cannot qualify new evidence by backfilling
identity. Reports with retained execution failures also cannot qualify merely
because their outer status says `ok`.

Tests cover all normalized built-in templates, each non-warning batch/step
field, nested object ordering, array/step order, Unicode/escaping, numeric JSON
round trips, tiny/subnormal/nearby floats, signed zero, integer/float tokens,
report compaction, invalid-batch preflight and retained report reloads.
Large array changes outside the report's three-item sample still alter the
fingerprint even when both compacted report payloads are identical.

## Actual Service Chain

`changed_real_solver_inputs_reject_old_reports_without_replaying_and_accept_new_results`
starts temporary source Orchestra/SQLite and two Rust Agents. Each successful
round creates a project/model through resolved bindings, submits an actual
512-element axial bar saved-version solve, waits and reads its result.

The first bar has length 1 m, area 0.01 square m, Young's modulus 210 GPa and
tip force 1000 N. Its displacement agrees with `F L / (E A)` within `1e-12`
relative error. The server retains 513 result nodes.

A parameter patch doubles the force to 2000 N. Pairing the first report with
the patched batch fails even after generic validation/warning metadata is
updated. Project lists, jobs, original result and Agent calculation counts
remain unchanged after refusal. There is no automatic recomputation.

An explicit second execution creates a distinct job and input fingerprint.
Its displacement is twice the first, within `1e-12` relative error. The second
batch/report/evidence is written, reloaded and verified against the first
evidence. The original first job/result is unchanged, the job count is two,
and the two Agents report exactly two calculations after verification.
Temporary processes and state are cleaned through the existing fixture guards.

The existing native CLI two-round controlled health-service test now asserts
both report fingerprints and their change after the parameter patch. It remains
a transport/evidence fixture, not a physical calculation test.

KCore's new regression changes a baseline input from `1.0` to its next
representable float. Legacy batch indexing still finds the retained batch,
but the new exact fingerprint prevents export before a file is produced.
Existing KCore complete-chain, native binary, patch-lineage and resealed-report
tests continue to pass.

## Verification

The selected full-suite regression passes 859 tests, without failures or ignored
tests in those runs:

- Native Headless SDK: 499 unit and 25 integration tests.
- Protocol: 114 unit and seven integration tests.
- KCore: six unit and 12 integration tests.
- Selected CLI suites: 90 tests, including all 51 actual-service tests in
  `headless_live`, three research-round tests and 16 execution-posture boundaries.
- Standalone Rust controller SDK: all 106 integration tests.

The repair adds 12 native SDK tests, one actual-service test and one KCore test.
Earlier focused/diagnostic runs are not counted again. Native SDK, Protocol,
KCore and CLI all-target Clippy pass with warnings denied.

The active workflow qualification requires the new precision, stale-report,
compaction, metadata and retained-failure anchors. Its minimum is raised to
499 SDK unit tests plus 16 CLI boundary tests, totaling 515 under its existing
largest-test-binary-per-suite counting rule. Repeated qualification runs are
not added to the separately summed 859 regression tests.
The updated gate and all eight new required anchors pass: 515 counted tests
across both suites. Its retained development report passes re-verification.

Formatting, diff whitespace, API surface, topology, matrix, extension standard,
tensor, documentation book/inventory, organization and version-line audits pass
with their applicable self-tests. All 224 exact version contracts remain 3.5.0.
Tensor posture remains zero structural, four maturity and 19 evidence-grade
gaps, with 14 P0 gaps and Daji status blocked; this scoped fix does not remove
unrelated readiness gaps.

Tensor evidence is scoped to verified native source contracts for
`sdk-headless`, not operational/installed or numerical qualification promotion.
The SDK guide is 1999 lines; the new focused guide avoids crossing its 2000-line
limit. New/modified source files remain below 800 lines.

## Remaining Boundaries

Input identity is not authenticated execution: a caller can fabricate both
report and fingerprint. It does not hash resolved external saved-model contents
or certify an Agent, deployment, solver, objective or scientific conclusion.
The bar's closed form independently checks this fixture only.
Old v1 research-series archives without captured report identity are still
readable as data, but now fail strict KCore semantic verification. They are not
silently upgraded or backfilled into new evidence.

Legacy TaskIR, parameter-patch receipts, batch/lineage hashes and their number
normalization are deliberately unchanged. A tiny-change multi-round patch may
still be refused as unchanged; the new gate prevents stale-result admission
but is not a versioned lineage migration. No benchmark or 1M-scale acceptance
is inferred from bounded serializer buffering or small source fixtures.

## Reproduce

```text
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk -p kyuubiki-protocol -p kyuubiki-kcore
cargo test --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test headless_live --test headless_research_round --test headless_execution_posture --test headless_artifact_generation --test headless_output_paths --test headless_material_results
cargo test --offline --manifest-path sdks/rust/Cargo.toml
cargo clippy --offline --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk -p kyuubiki-protocol -p kyuubiki-kcore -p kyuubiki-cli --all-targets -- -D warnings
CARGO_NET_OFFLINE=true make qualify-headless-workflow
```

Actual-service tests need permission to bind temporary loopback ports and start
local fixture processes. They require no external network or server deployment.
