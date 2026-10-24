# Workflow Execution Evidence Consistency

Date: 2026-10-24

Source base: `308eb93c`, with the previous selected-input and output-retention
changes preserved in the working tree. Package metadata remains 3.7.1; the last
version-named Git checkpoint is `daji 3.7.3`. No version bump, commit, deployment
or installed application rebuild is performed in this round.

## Reproduced Control-Plane Gaps

Two new Elixir regressions fail before repair while the named-input control
passes. These are execution-record defects, not reproduced wrong solver values.

For a condition choosing `if_true`, Orchestra correctly publishes only that
artifact, but its completed-node trace lists all declared ports: `if_false`,
`if_true` and an unused third port. The trace therefore disagrees with actual
artifacts and lineage. Rust already records only the chosen publication.

For a first-available transform whose selected payload fails the generated
string budget, Orchestra correctly fails the node under explicit skip policy,
but its failed-node trace lists both available sources as consumed. The selected
source and attempted computation must remain identifiable without claiming an
unused backup. Prior Rust selected-input controls already enforce that rule.

## Narrow Record Repair

The existing failed-node recording path now reuses the same kind-aware input
resolver used for execution binding, publication lineage and successful traces.
No new input-binding API or recovery subsystem is introduced. Named-input
operators still record every binding, including repeated uses of the same
source; only the first-available operator selects a single contributor.

Completed traces now filter candidate publication keys against the actual
post-publication artifact map before response shaping. The map is not copied
or retained in another snapshot. This avoids duplicating condition-branch
selection logic and preserves present false, zero and null values by key
membership rather than payload truthiness.

The graph format, scheduling, callbacks, payload values, branch decisions,
skip/fail policy, generated-output budgets and persisted result schema do not
change. Corrections apply to new runs; historical stored traces are not silently
rewritten. No fake publication is added to make an incorrect trace appear true.

## Shared Regression Fixture

`tests/fixtures/workflow-execution-evidence.json` defines one graph and four
small literal cases. Rust and Elixir run the same fixture against independently
stated branch and selected-source expectations. This is shared-oracle
regression testing, not a live connection between those two runtimes.

- Conditions cover true, false, zero and null, reordered node declarations,
  reversed fallback priority and an unused condition port.
- Every node's produced keys must agree with its actual publication lineage;
  failed and skipped nodes must have no produced keys.
- Failed fallbacks cover either selected source, duplicate-source edges,
  ordinary/nested skip policy, dependent skips and independent raw outputs.
- Failed named-input transforms retain both source bindings, even with aliases.
- Rust additionally covers cached/ephemeral condition paths and all/output-only/
  no-artifact projection. These do not imply equivalent cache implementation
  across runtimes.
- Public Orchestra API routing covers synchronous results and asynchronous
  jobs. SQLite result readback must preserve returned traces, branch decisions,
  lineage and projected payloads. Compact failures keep node identity and cause.

The API tests use the local Plug router and the test application's disposable
SQLite database, not an installed service, TCP transport, remote Agent, physical
server or database restart. The two bad trace shapes are never accepted as
equivalent merely because returned payload values are equal.

## Repeatable Entry

`make test-workflow-execution-evidence` executes six Elixir test functions and
three Rust test functions using the same fixture. The component validation
profile additionally runs the six selected-input and five output-retention
controls. Its 20 tests overlap ordinary suite runs and are not extra independent
qualification evidence.

## Completed Local Checks

macOS AArch64 uses the pinned Rust 1.88.0 toolchain with locked offline
dependencies and local Elixir 1.19.5 on OTP 28. Existing build caches are reused.

| Scope | Result | Exclusions |
| --- | --- | --- |
| Full web suite | 3,475 tests, zero failures | 77 pre-existing skips |
| Engine library | 712 passed | 1 pre-existing ignored test |
| Six selected Engine integration targets | 25 passed | Not all integration targets |
| New component profile | 20 tests passed | Overlaps the suite counts above |

The selected Rust integration targets are `shared_execution_evidence`,
`shared_condition_contract`, `shared_named_input_contract`,
`shared_branch_recovery`, `workflow_selected_input` and
`workflow_output_retention`. The two deliberately failing pre-fix Elixir tests
and earlier report totals are not added to these final-run counts.

Engine all-target Clippy with warnings denied, Rust/Elixir formatting, Make
module validation, documentation book/inventory and diff checks pass. All 110
validation-profile definitions validate; only the new profile is executed as a
profile in this round. The organization gate preserves the 800-line source and
2,000-line document limits without an exception.

The coverage tensor passes structural and command checks with 13 modules,
11 paradigms, zero structural gaps, two maturity gaps, 23 evidence-grade gaps
and 18 P0 gaps. Daji remains blocked. No numerical accuracy, distributed
recovery or release-readiness promotion is inferred from this component
contract. Current working-tree changes are not claimed as remote CI evidence.

This is bounded Rust/Orchestra execution-trace consistency and local SQLite result readback evidence, not authenticated provenance, process-restart or power-loss qualification, distributed recovery, installed acceptance or numerical certification.
