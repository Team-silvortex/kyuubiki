# Headless Research Plan Preflight

Date: 2026-10-07. Base `7276aa6c` has subject `daji 3.5.1`; source/package
metadata remains 3.5.0 and installed baseline remains 3.4.0. Earlier uncommitted
receipt ownership, publication, path and generation repairs are retained. This
follow-up performs no App install, version bump, commit or push.

## Repair

The earlier native CLI checks the presence of a previous evidence file and patch,
but not the complete baseline/continuous lineage contract before dispatch. A
metric can also reference a nonexistent zero-based step. Such static mistakes
are rejected by evidence generation only after service requests have completed.
The new nonexistent-step CLI regression first failed against that behavior: its
completed report contained one executed `service_health` step and status `ok`,
instead of an invalid zero-step preflight report.

Public Rust SDK `validate_headless_research_round_plan` now checks an effective
batch/spec, workflow association, metric step bounds and baseline/continuous
lineage without an executor or report. It borrows inputs without mutating them.
The SDK evidence builder reuses the same static target and lineage helpers and
still separately validates actual execution and finite numeric observations.
No numerical algorithm, TaskIR/schema version or Engine/Agent authority changes.

Checks include:

- Spec syntax and valid batch/workflow identity. Metric pointers use zero-based
  array positions, not a batch step's one-based action index.
- Round 1 has no previous evidence or patch receipt. Its preflight skips an
  unused canonical batch hash, while completed evidence still records its hash.
- Later rounds require structurally valid previous service evidence, the next
  iteration, a changed round ID and the same workflow.
- The parameter receipt must target the effective current batch, change inputs
  and begin at the previous batch hash. Input edits after applying a receipt
  require renewed validation, not reuse of a stale receipt.

The native research CLI calls this API after patching and timeout overrides,
before constructing its service executor or issuing workflow requests. A scoped
preflight prefix retains `research_round_validation`, stage `research_round` and
non-retryable classification even if a pointer or file label includes timeout,
artifact-limit or decode-error wording. Rejection produces an invalid zero-step
receipt and does not touch the prior derived evidence.

## Tests

Twelve new tests comprise six SDK static-policy tests, one CLI diagnostic unit
test, four controlled-service CLI tests and one actual-service test. Loops cover
valid unchanged/continuous inputs, zero-based first/last positions, nonexistent
and huge indices, invalid pointer escapes/leading zeroes, baseline patching,
missing previous evidence/receipt, invalid evidence mode/qualification, crossed
workflow, skipped or reused rounds, disconnected hashes, invalid receipt target
and edits after patch application. Input snapshots remain unchanged.

The controlled HTTP spy records zero requests for static invalid cases. Each
failure has zero executed steps/jobs, a saved failure report matching stdout,
non-retryable research-stage diagnostics and unchanged workflow/previous/old
evidence. Valid first and patched contiguous rounds each issue exactly one
request and retain matching before/after lineage hashes. These health-service
metrics and some synthetic previous evidence are contract fixtures only, not
material or producer-authenticity qualification.

A temporary Orchestra/SQLite and two actual Rust Agents retain a baseline
project/saved bar model. A workflow would first create a project and then submit
a saved-version bar calculation. Both a nonexistent metric step and an invalid
first-round patch are refused beforehand. Project/job state stays byte-for-byte
JSON-equal to its baseline, the job list is empty, both Agents stay accepting
idle and combined calculation count stays zero. Original workflow/patch bytes
and prior evidence are retained; optional failure run files remain inspectable.
Owned processes and temporary data are cleaned by the fixture.

Static acceptance does not predict result fields. SDK tests still refuse missing,
null, string, Boolean, array or object metrics after execution. Earlier artifact
generation and real retained-result recovery regressions remain in the selected
suite; a successful static plan cannot bypass those gates.

## Verification

Selected source regression passes 942 tests with zero failures, ignored tests or
filtered tests in the final full selected runs:

- Headless SDK: 451 unit and 25 integration tests; the unit suite takes 34.23 s.
- Protocol: 114 unit and 7 integration tests.
- CLI: 182 ordinary CLI and 43 Headless unit tests plus 103 selected integration
  cases. All 47 actual Orchestra/Agent tests pass in 21.31 s; all four controlled
  research-preflight tests and the earlier artifact-recovery tests also pass.
- KCore consumer regression: all 17 tests pass, including retained research-series
  exchange validation. This does not qualify arbitrary external archives.

Clippy for CLI/SDK/Protocol tests passes with warnings denied; workspace formatting
and `git diff --check` pass. Runtime API-surface, module topology, module/function
matrix, extension-standard, coverage-tensor, book/inventory, organization and
version-line checks pass, including their applicable self-tests. The matrix stays
13 modules by 11 paradigms. Tensor status retains zero structural, four maturity
and 19 evidence-grade gaps, 14 P0 gaps and blocked Daji status. There is no grade
promotion. Documentation checks cover 26 HTML files; all 224 exact source version
contracts match 3.5.0. Organization has zero tracked debt under the 800-line source
and 2000-line document limits.

Reproduce from `workers/rust` with local service test permissions:

```text
cargo test --quiet --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-cli --bin kyuubiki-cli --bin kyuubiki-headless --test headless_live --test headless_artifact_generation --test headless_research_preflight --test headless_task_completion --test headless_cli_surface --test headless_parameter_patch --test headless_research_round --test headless_execution_posture --test headless_wait_recovery --test headless_output_publication --test headless_output_paths -- --test-threads=4
cargo test --quiet --locked --offline -p kyuubiki-kcore -- --test-threads=4
cargo clippy --quiet --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

## Limits

This is macOS ARM64 source `verified` execution/contract/recovery. It is not new
scientific, scale, installed GUI/Agent, Linux/Windows, authenticated producer,
durable recovery or Python/Elixir parity qualification. Tensor gaps remain open.
SDK consumers explicitly opt into this optional research policy before execution;
the general batch executor does not implicitly acquire a research specification.

Previous evidence is structurally checked, not independently authenticated. Metric
units/objectives and scientific meaning still require the study's own validation.
Preflight cannot predict a result field's existence, numeric type or physical
validity, nor a service's future availability. Material-study candidate/result
completeness is separate work. Actual report/result/lineage validation and explicit
recovery remain mandatory; no automatic execution/replay authority is added.

This rejection does not promise zero file writes: an explicitly requested invalid
run report may be published, and an applied patch receipt may already be written.
It does prevent service-side workflow effects for the checked static failures.
No multi-file transaction, untrusted-directory sandbox or aggregate RSS/CPU bound
is claimed.
