# Workflow Result Admission Regression (2026-09-24)

## Scope

Source-level continuation of the [spring/contact committed-state recovery](nonlinear-spring-recovery-20260924.md).
No solver equation, convergence tolerance, scheduling policy, runtime package,
or installed application was changed in this round.

This is bounded local result-admission validation, not complete research-workflow or industrial physics qualification.

## Reproduced Failure

The returned committed state can be internally consistent but incomplete:

- A hardening spring with one Newton correction returns the initial state,
  `converged=false`, `achieved_load_factor=0`, and zero root residual/displacement.
- A gap-contact model with four load steps and one correction per step returns
  its last equilibrated half-load state, `converged=false`, and zero root residual.

Previously the guard ignored nonconvergence, structural quality could report
`ready=true`, and pair/candidate comparison could select the failed result
because its displacement or score was smaller. Generic summary extraction also
removed the status before a later consumer could see it.

The initial focused regression, with valid solver inputs, had **15 failures and
3 passing controls** before the admission fix. Those failures demonstrated
incorrect successful assessments or reductions, not compile/fixture failures.

## Admission Contract

`workflow_result_admission.rs` is shared by result-consuming operators, not the
generic graph scheduler or the physical solver. It checks the current result's
`converged` marker and the material-frame contract's `stability_result.converged`.

- Explicit `false` rejects assessment/reduction with the operator, source path,
  and available achieved-load factor in the error.
- A present non-boolean marker or non-object stability wrapper is an error.
- A missing marker remains compatible with existing metrics-only and linear
  results. Absence is not a new assertion that the physical solve converged.
- The load factor is diagnostic context only. No universal `factor == 1` rule
  is imposed on operators with non-unit or path-controlled target loads.
- Historical trial steps, arbitrary metadata and entire field arrays are not
  recursively scanned. The additional check is independent of mesh size.

Covered entry points:

- Guards, quality scores and pair comparisons for structural, thermal, acoustic,
  modal, dynamic, electrostatic, magnetostatic, CFD and transport domains.
- Generic result-summary, field-statistics and field-hotspot extraction.
- Summary merge, compare, aggregate, normalize, select-best and tolerance validation.
- Composite quality sources, including validation-derived quality evidence.
  Candidate ranking uses its existing rejection path; an invalid candidate does
  not become a winner, the ranking remains incomplete and next-round planning
  returns `replan` rather than declaring the study complete.

This is an intentional fail-closed change for these consumers. Raw results
remain available through direct solve results, raw workflow output branches and
JSON export; partial-state diagnostic work must not mistake them for full-target
quality evidence. No new snapshot or alternate result schema is introduced.

## Workflow Recovery

The public graph regression executes both real solver families with and without
a summary-extraction stage:

1. The default policy fails at the rejecting consumer with a convergence error.
2. Explicit `on_error: "skip"` marks that consumer failed and skips its dependent
   decision branch, without emitting a quality artifact.
3. Independent work and a separate raw solver-output branch complete. The raw
   result retains the committed load and failed-trial diagnostics.
4. Replaying with an adequate correction budget reaches full load and completes
   both the direct scoring and extract-then-score chains.

The original solver-only workflow contract still permits incomplete diagnostic
results. A graph node completing execution is not itself a physics verdict.

## Verification

- Focused consumer regression: **18 passed**, including table-driven checks of
  all 27 registered domain guard/quality/pair entry points.
- Public workflow recovery regression: **3 passed**, each exercising both spring
  and contact and both direct and summary-first paths.
- Protocol, solver, engine and Rust headless SDK libraries: **1,202 passed,
  7 pre-existing ignored**, no failures.
- Retained native validation profile: **25 passed** across two commands/five
  suites, including four pre-existing raw-result workflow tests.
- Optimized release workflow regressions: **5 passed**, including the three new
  recovery tests and two pre-existing raw-state tests.
- Engine all-target Clippy with warnings denied, targeted formatting and diff
  checks passed. Operator-profile, tensor, project-organization and documentation
  inventory gates passed; source files remain within 800 lines and docs within 2,000.

The 21 added tests are included in these totals, not additive certification cases.
Reproduce the retained component profile with the native runner:

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-result-admission --out tmp/workflow-result-admission.json
```

The profile also reruns existing raw spring/contact workflow tests. Machine-local
logs and generated JSON reports are not release evidence or tracked artifacts.

## Limits

- Custom transforms, external summaries with removed/forged markers, unrelated
  domain-specific reducers and bridges are not certified by this check.
- Missing numerical metrics or malformed threshold/criterion configuration have
  separate validation semantics; this round does not certify them exhaustively.
  The [guard/pair follow-up](workflow-guard-validation-20260924.md) closes these
  bounded consumer paths without extending that claim to all metric reducers.
- This does not prove nonlinear convergence, solve accuracy, target-load
  consistency of arbitrary JSON, durable restart, remote deployment, or GUI behavior.
- No large-scale benchmark, server execution, app rebuild, installation, commit
  or push was performed for this round. Qualification remains blocked pending
  the broader evidence requirements; no overall tensor maturity grade is raised.
