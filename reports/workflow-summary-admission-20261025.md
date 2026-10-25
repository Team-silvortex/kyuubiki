# Workflow Summary Admission Parity

Date: 2026-10-25. Source base: `e347cf8c`, with the local named-source integrity
repair retained. Package metadata remains 3.7.1. No version bump, deployment,
desktop rebuild or historical-result rewrite.

## Reproduction And Repair

Preserving a complete named source exposed the next admission boundary:
Orchestra's summary merge, comparison, aggregation, selection, normalization
and result-summary extraction accepted explicit failed or malformed final
status. Reduction could erase that status before downstream consumers saw it.
The initial Elixir regression had 13 tests and nine failures before the repair.

Both runtimes also omitted root admission for summary collections and counted
the `stability_result` object as a candidate. Two native regressions failed
before repair: a rejected root status still produced an aggregate, and status
metadata changed the input count from two to three. The same metadata could
compete for selection when it contained the configured metric.

The six Orchestra entrypoints now reuse the existing reporting admission
checks. Both collection implementations validate the payload root and exclude
the reserved `converged` and `stability_result` fields from candidate enumeration.
Each actual candidate still passes the existing final-status contract before
aggregation or scoring. No solver algorithm or numerical tolerance changes.

## Contract Boundaries

- Explicit `converged: false`, nonboolean convergence, malformed stability
  objects and failed/malformed nested final convergence are rejected.
- Absent status remains valid for metric-only inputs; it does not certify
  convergence. An empty stability object has no explicit failure marker.
- Failed historical trials do not override final success. The checks do not
  recursively scan arbitrary metadata or reinterpret achieved load factors.
- Reserved collection status fields cannot increase the candidate count, win
  selection or form a collection by themselves. Other object keys remain
  candidate identifiers, not arbitrary collection-level metadata.
- Raw failed results remain exportable for diagnosis. Explicit skip policy
  records the failed node and skips its dependent assessment output, without
  publishing assessment artifacts or lineage. A corrected fresh execution is
  independent of the retained failed result; this is not checkpoint resume.

## Shared Regression Entry

`make test-workflow-summary-admission` passes 17 Elixir/API tests and five Rust
integration tests. The existing `workflow-result-admission` component profile
includes this entry; counts overlap broader suites and must not be added.

The shared fixture covers six real registered operators, 12 invalid status
variants and five valid controls at the root and applicable named-source
positions. Positive assertions include independently specified metrics and
the winning source, not just two implementations agreeing. Graph checks cover
fail-fast, explicit skip, preserved raw outputs and clean fresh execution;
Rust also covers full/output-only/no-artifact projections. Local API checks
exercise synchronous rejection, asynchronous failure and recoverable jobs,
full/compact responses and disposable SQLite result readback. Fail-fast jobs
retain their failure/recovery envelope, not a successful assessment.

## Verification Scope

The full Web/Orchestra suite passes 3,503 tests with zero failures and 77
pre-existing skips. The Rust Engine library and eight selected integration
targets pass 747 tests with zero failures and one pre-existing ignored library
test. All-target Engine Clippy with warnings denied, Rust/Elixir formatting,
book/inventory checks, the 800/2000-line organization audit and diff checks pass.
The `workflow-result-admission` profile was executed successfully, not merely
registered. Validation is local macOS AArch64 with Rust 1.88.0 and Elixir
1.19.5/OTP 28.

The existing cross-runtime reporting tensor claim is extended without changing
its grade. The tensor check retains 13 modules, 11 paradigms, zero structural
gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0 gaps; Daji remains
blocked. These local contracts do not close the open operational coordinates.

This is bounded local summary-admission evidence, not whole-runtime equivalence, distributed recovery or industrial physics qualification.
No authenticated multi-host execution, process kill, power loss, remote CI,
installed desktop acceptance or all-operator numeric-range qualification is
claimed by this run.
