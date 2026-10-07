# Headless Post-Run Artifact Generation

Date: 2026-10-07. Base checkout `7276aa6c` has subject `daji 3.5.1`;
actual source/package metadata remains daji 3.5.0. Installed baseline is 3.4.0.
This source follow-up preserves earlier uncommitted report ownership, publication
and path-protection changes. No App rebuild/install, version bump, commit or push
is performed.

## Repair

Two new regressions first failed against the earlier CLI:

- A shortened material workflow completes its preview steps, then fails report
  construction with `expects 3 result payloads, received 1`. The old CLI returns
  before printing its completed run, classified as `headless_command_failed`.
- A successful controlled service call lacks a requested metric whose JSON
  pointer includes `timed out waiting for job`. The old CLI reports a retryable
  `job_wait_timeout` and advises resuming job polling, although no job wait failed.

A small native post-run module now generates and publishes the derived artifacts.
Material-report construction errors enter the same actual-run receipt preservation
path as research-evidence construction errors. Only generation errors receive the
dedicated prefix and `report_generation_failure`; existing publication/path errors
are propagated unchanged. Its stage uses the existing `artifact_output` value.
No SDK API, schema version or numerical algorithm changes.

JSON stdout or ordinary text stdout retains the completed run when writable,
including actual status, successful-step count and job identity. Previously
published run files remain available. The invocation exits nonzero and stderr
explicitly denies automatic step/batch replay. Prefix classification takes
priority over timeout, execution, parameter, artifact-limit and research words
in an artifact label or metric pointer.

Successful computation is not successful artifact generation or a qualified
study. Missing/non-numeric metrics and inconsistent material result sets remain
errors; no fake evidence, relaxed metric gate or overwritten prior derived file
is introduced. Real execution/preflight failures keep their existing categories.
No execution authority moves into the CLI artifact module or out of Engine/Agent;
Solver retains its calculations.

Later source follow-up: [material preflight](./headless-material-preflight-20261007.md)
now rejects the shortened material plans used below before execution. Their CLI
regressions were upgraded to invalid zero-step receipts, not removed or bypassed.
An additional post-run unit test retains incomplete-result-set rejection. The
earlier measured results below remain historical; runtime research-metric failure
and actual retained-result recovery continue to test completed CLI receipts.

## Tests

Seven new tests comprise one diagnostic-priority unit case, five CLI generation
cases and one actual-service recovery case. Additional loops cover:

- Zero or one material result instead of the expected three, both with and
  without an optional run file; completed stdout/run file and original workflow
  are retained, and prior material output is unchanged. These mock cases check
  plumbing only, not physical material qualification.
- A missing metric with timeout text cannot become retryable job polling.
  The controlled HTTP service observes one explicit health request.
- Null, numeric string, Boolean, array and object metrics remain non-numeric.
  Completed reports survive, while no new research evidence or staging appears.
- Non-JSON output includes actual successful steps but no material winner summary.
- An actual controlled HTTP 500 remains an execution failure, never a generation
  failure or successful completed run. Its evidence file remains absent.

The actual-service fixture starts temporary Orchestra/SQLite and two Rust Agents.
It creates a baseline project/saved model through the SDK, submits one 512-element
bar calculation through the research CLI, then fails evidence construction on a
missing displacement metric. The one-step completed run remains on stdout and
in its run file. Stored job/result data are retained, including 513 original nodes;
tip displacement independently matches `FL/EA` to relative `1e-12`.

The combined solve/wait step wraps its fetched result: the correct scalar pointer
is `/steps/0/result_preview/result/result/tip_displacement`. During fixture
development, omitting that second result layer was rejected by the same numeric
gate; correcting the mapping did not weaken validation or require another solve.

The effective batch is rendered without execution. Public SDK
`build_headless_research_round_evidence` and `verify_headless_research_round_evidence`
explicitly rebuild and verify derived evidence from the retained typed run and
corrected spec. The recovered metric equals the original independent physical
readback. Stored job/result values remain unchanged, the database contains one
job, combined Agent calculation count remains one, both Agents return to accepting
idle state, and the original workflow/old evidence file are unchanged. Owned
processes and test data are cleaned. The evidence's `qualified` field describes
the existing builder contract, not promotion of global scientific evidence grade.

## Verification

Selected source regression passes 913 tests, with zero failures, ignored tests or
filtered tests in the final full selected runs:

- Headless SDK: 445 unit and 25 integration tests; the unit suite completes in
  34.19 seconds.
- Protocol: 114 unit and 7 integration tests.
- CLI: 182 ordinary CLI and 42 Headless unit tests, plus 98 selected integration
  tests. The latter include all 46 actual Orchestra/Agent cases, completing in
  23.46 seconds, and all five new artifact-generation cases.

Clippy for CLI/SDK/Protocol tests passes with warnings denied. Workspace formatting
and `git diff --check` pass. Runtime API-surface, module topology, module/function
matrix, extension-standard, coverage-tensor, documentation-book/inventory,
organization and version-line checks pass, including their applicable self-tests.
The matrix remains 13 modules by 11 paradigms. The tensor retains zero structural,
four maturity and 19 evidence-grade gaps, 14 P0 gaps and blocked Daji status; no
maturity or evidence-grade promotion is claimed. Documentation checks cover 26
HTML files; all 224 exact source version contracts match 3.5.0. Organization has
zero tracked debt under the 800-line source/2000-line document limits.

Reproduction from `workers/rust`:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_artifact_generation --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery --test headless_output_publication --test headless_output_paths -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

Evidence is macOS ARM64 source `verified` execution/contract/recovery, not deployed,
authenticated/durable, installed GUI/Agent, Linux/Windows, Python/Elixir parity or
scientific/scale qualification. Independent SDK consumers retain their own
generation error and storage policy. No universal CLI replay command is added.

The study remains incomplete when necessary results are missing or invalid.
Repair extraction/mapping only against actual retained results; changing physical
inputs cannot retroactively qualify an old calculation. Additional calculations
require an explicitly authorized new plan, not automatic original-workflow replay.
Broader static pre-dispatch material/metric/lineage completeness and data-provenance
qualification are separate work; this repair handles failures after a run.

An existing artifact file can belong to an earlier run: consume exit status and
stderr and verify batch/report hashes and lineage, rather than trusting presence
or run `status: ok`. Already published artifacts are not rolled back. This is not
a multi-file transaction, crash/power-loss checkpoint, total RSS/CPU cap, throughput
benchmark or hostile-directory sandbox. If stdout fails too, inspect the owning
service records; the derived-output failure cannot grant replay authority.
