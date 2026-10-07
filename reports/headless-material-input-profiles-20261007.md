# Headless Material Input Profiles

Date: 2026-10-07. Fixed built-in material reports now require candidate input
profiles matching the constants and assumptions used by their report builders.
This prevents edited solves from being ranked with stale factory parameters.
Generic Headless research remains free to edit its own inputs and report mapping.

Base `7276aa6c` has subject `daji 3.5.1`; source/package metadata remains 3.5.0
and installed baseline remains 3.4.0. Prior uncommitted repairs are preserved.
This follow-up does not install Apps, bump versions, commit or push.

## Fixed Candidate Assumptions

The previous candidate-chain preflight accepted a different candidate's model
under an unchanged candidate label. A new regression reproduced that behavior
before repair. SI consistency alone also accepted coordinated relative/absolute
permittivity edits, even though dielectric loss, mass and breakdown metrics still
use original candidate constants. The original coordinated-edit unit assertion
is corrected to distinguish unit consistency from report applicability.

The public `validate_material_report_compatibility` now adds a private fixed-input
check after candidate-chain/batch validation and the existing SI check. It compares
models, constitutive properties, geometry, boundaries and all composite loss,
feedback and conduction inputs against the selected candidate's workflow factory.
Research properties, material identities and card references must agree. Unknown
physical/model/coupling keys fail closed instead of silently redefining a profile.
Diagnostics include the step, candidate and escaped JSON pointer of the first
mismatch; they do not dump a full model or reinterpret field text as retry advice.

Reference solve profiles are generated lazily once per selected built-in study,
with 15 finite candidates across five caches. Caller models/results are borrowed
for comparison and never cached. A test verifies repeated access shares the same
reference and the full reference set's encoded JSON stays below 128 KiB; this is
not an RSS limit or measured throughput benchmark. Integer comparisons preserve
integer identity; float comparison admits only 8 machine epsilons relative to
magnitude, without an absolute floor. It accepts JSON roundoff, not a scientific
or solver tolerance, and rejects zero substitutes for tiny physical parameters.

Study aliases, root project/version/case context and annotations remain allowed.
Runtime association checks still apply to context. Research display labels,
notes and objectives do not redefine canonical report identity. Unknown research
properties must not silently override fixed report constants. Generic batch
validation/execution, parameter patching and existing supported materialized
candidate/report builders remain independent of this optional fixed-report gate.
For changed physical inputs, choose an appropriate custom research specification
or a supported materialized candidate report, not the original fixed ranking.

## Regressions

Eleven new SDK tests cover transplanted models, constitutive drift without changed
model IDs, geometry/boundary/thickness edits, missing/null/extra models, altered
mass/strength/material-card metadata, eight composite model/coupling parameter
paths and unknown coupling overrides. Consistent SI dielectric edits are rejected
as fixed-profile mismatches; uncoordinated edits retain their earlier SI diagnostic.
Aliases, serialized numeric inputs, annotations and runtime context still pass
without mutation. Generic edited workflows continue to execute as explicit mock
adapter tests, while the fixed report gate rejects their changed assumptions.
Existing materialized builder tests remain in the full SDK suite.

Two new controlled CLI tests loop all five studies for model transplant, thickness,
geometry, metadata and unknown-field drift, plus coordinated SI changes. The
dielectric transplant fixture copies the donor's relative value too, proving the
new profile gate rather than only the existing SI check. Rejections have zero
requests, steps and jobs, exact non-retryable material diagnostics, unchanged
source/prior material files, matching saved failure reports and no staging residue.
The recovery action directs changed-input research away from fixed constants.

One new actual-service test shares the retained fixture with the earlier chain
test. Temporary Orchestra/SQLite and two Rust Agents reject 15 changed-physics
plans across all five studies. Each would first create a project, then execute
three candidate chains. Projects/jobs remain JSON-equal to baseline, the job list
stays empty, both Agents remain accepting idle and total calculations stay zero.
Source/prior material files survive, requested run files match stdout, and owned
temporary data/processes are cleaned up. This is rejection qualification, not a
new successful five-study physical simulation or numerical certification.

## Test Service Repair

The selected artifact-generation regression also exposed a test-only race:
its health fixture accepted a client from a nonblocking listener and immediately
unwrapped a single read. A delayed fragmented client reproduced `WouldBlock`
and a broken pipe. Previously, a secondary panic while joining the failed server
during test unwinding could abort the entire test process.

The accepted stream now explicitly switches to blocking reads, and a bounded
reader waits for complete request headers with a single two-second deadline and
4 KiB cap. Interrupted reads do not reset the deadline. The fixture checks the
request only after the headers complete. Cleanup still joins and reports thread
failure on normal exit, but does not add a second panic during an existing unwind.
A new delayed three-fragment test makes three requests and checks three replies
and exact request count; the regression first failed before the reader repair.
This changes test infrastructure, not native production HTTP transport policy.

## Verification

Final selected source regression passes 970 tests with zero failures, ignored
tests or filtered tests in the full selected runs:

- Headless SDK: 469 unit and 25 integration tests; the unit suite takes 34.02 s.
- Protocol: 114 unit and 7 integration tests.
- CLI: 182 ordinary CLI and 45 Headless unit tests plus 111 selected integration
  cases. All 49 actual Orchestra/Agent tests pass in 29.11 s. All five controlled
  material tests and all six artifact tests pass, including the delayed client.
- KCore: all 17 consumer tests pass, without new archive qualification.

Clippy for CLI/SDK/Protocol tests passes with warnings denied, and workspace
formatting and `git diff --check` pass. Runtime API-surface, module topology,
module/function matrix, extension-standard, coverage-tensor, book/inventory,
organization and version-line checks pass with applicable self-tests. The matrix
retains 13 modules by 11 paradigms and the tensor retains zero structural, four
maturity and 19 evidence-grade gaps, 14 P0 gaps and blocked Daji status. No grade
promotion occurs. The doc check covers 26 HTML files; all 224 exact version
contracts match 3.5.0. Organization retains zero tracked debt under source 800 /
document 2000 limits, and new modules are individually below the source limit.

The active Headless workflow qualification contract now names the corrected SI /
fixed-profile test and also requires the coordinated-SI and roundoff-only tests.
Historical 2.13.1 evidence keeps its original names and measurements. A fresh
native contract execution passes both suites with all required anchors present,
zero failures and zero ignored tests. Its existing largest-test-binary-per-suite
counting rule reports 485 tests (469 SDK unit tests and 16 CLI boundary tests),
rather than summing every test binary. SDK and CLI suites take 54.060 s and
0.463 s respectively. This separate repeated run is not added to the independently
summed 970 selected tests above. The overwriteable local report is
`tmp/headless-workflow-qualification-report.json`; it is not retained as a new
versioned artifact or used to promote qualification.

Reproduce from `workers/rust` with local service fixture permissions:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_artifact_generation --test headless_material_preflight --test headless_research_preflight --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery --test headless_output_publication --test headless_output_paths -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-kcore -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Qualification Limits

This is macOS ARM64 source `verified` execution/contract/recovery evidence with
no qualification promotion. Matching a built-in profile does not authenticate
material constants, a producer, a run receipt or a service's real physical input.
Independent result checks, numerical/physical validation, research lineage and
retained-result association remain mandatory. Literal or standalone report
builders have no effective batch to check: their callers retain those duties.

General arbitrary-candidate reports, authenticated provenance, installed GUI/
Agent, Python/Elixir parity, Linux/Windows, durable recovery and scale remain
separate qualification. Solver keeps numerical ownership and Engine/Agent keeps
execution authority; this SDK policy adds no execution, replay or filesystem
sandbox power. It changes no schema/protocol version and adds no data backups.
