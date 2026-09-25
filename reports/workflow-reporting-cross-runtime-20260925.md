# Rust/Elixir Reporting Contract Alignment, 2026-09-25

## Scope

This is bounded local Rust/Elixir reporting-contract evidence, not whole-runtime equivalence or industrial physics qualification.

The [Rust field/bundle audit](workflow-field-bundle-integrity-20260925.md)
identified reachable Elixir implementations behind Orchestra graph execution
and local TaskIR execution. This round aligns `extract.field_statistics`,
`extract.field_hotspots`, `transform.compose_diagnostics_bundle` and
`transform.evaluate_diagnostics_bundle_guard`. It does not move computation
between hosts, introduce a new Rust process/NIF dependency, or change scheduling.

## Reproduction and Contract Changes

The initial 65 shared JSON cases all passed the checked Rust implementation.
Against old Elixir code, 58 of 65 failed. These are overlapping contract cases,
not 58 independent physics defects. Failures included silent sample/rule
filtering, malformed configuration defaults, missing metrics becoming passed
guards, nonconvergence disappearing, missing counts becoming zero, and unchecked
integer totals. BEAM arithmetic overflow escaped as an exception in some cases.

Additional compatibility differences included default hotspot selection using
the maximum instead of p90, percentile keys rounded to two decimal places,
null hotspot IDs being dropped, and absent prefixed count fallback.

The existing public Elixir facades remain. Their old permissive bodies and
now-unused helpers are removed; checked helpers, field reductions and bundle
validation live in three focused modules. Invalid explicit configuration reaches
the checks without `false || %{}` substitution. Domain solver equations,
operator SDK boundaries and generic summary reducers remain unchanged.

- Selected collections must be nonempty and every consumed row/field valid.
  Final convergence markers are checked without interpreting trial histories.
- Invalid explicit source/field/prefix/percentile/threshold/sampling options
  return errors. Valid default and threshold-precedence behavior is retained.
- Required sums fail safely; scaled standard deviations, stable interpolation
  and hotspot running means handle the retained large/small numerical cases.
  Arithmetic exceptions are caught only at checked numeric operations.
- Bundle sources and metadata follow the Rust selection contract. Canonical
  counts take precedence, unknown counts remain null and totals enforce u64
  range even though Elixir supports arbitrary-size integers.
- Bundle guards parse and evaluate every rule, use exact published fields and
  reject missing metrics/sources. All retained sources undergo convergence
  admission, not only the source mentioned in a rule.
- Valid threshold violations produce blocked/warning reports, not execution
  errors. Markdown now prints unknown counts as `null`, not blank or zero.

Two old positive fixtures were updated deliberately: the retention-options
fixture now contains a real zero measurement instead of metadata only, and the
generic-summary fixture has numeric evidence with a valid declared diagnostic
contract (or no diagnostic declaration). The shared negative cases independently
retain rejection of the formerly accepted malformed inputs.

## Verification

Shared fixtures live under `tests/fixtures/workflow-{field,bundle}-contract.json`.
Both harnesses check the same explicit expected output fields or error paths;
floating outputs use relative tolerance `1e-12`, with no absolute floor that
would accidentally accept a tiny nonzero result as zero. This is not a claim
that every output field, message string or IEEE bit pattern is identical.

The same `workflow-report-contract.json` graph executes field extraction,
bundle composition, guarding, report composition and JSON export in both
runtimes. Rust checks success/block decisions and corruption at all three
checked stages. Elixir checks six fault mutations per extractor, fresh corrected
replays and local TaskIR admission. The API repeats the 12 negative cases and
their corrected replays, checking 422 with no artifact payload versus 200 with
a real report. These calls use the in-process Plug router, not a deployed server
or remote Agent. Existing coupled/template/condition API tests remain controls.

The first normal Mix invocation stopped at the existing unversioned development
database protection. No existing database was deleted, migrated or bypassed.
Pure contract/graph/TaskIR tests use `--no-start`; API checks start the app with
a fresh disposable SQLite database in a uniquely named temporary directory.

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-reporting-cross-runtime-contract --out tmp/workflow-reporting-cross-runtime-contract.json
```

Completed checks on local macOS ARM64, Elixir 1.19.5 / OTP 28:

| Check | Result |
| --- | --- |
| Shared field/bundle fixtures | 65 cases pass in both languages; Elixir checks direct and dispatcher entrypoints |
| Native validation profile | 98 tests pass: 94 Elixir + two Rust fixture harnesses + two Rust graph tests |
| Focused graph/coupled/template/condition API suite | 10 tests pass, including the 12 corrupt reporting requests and corrected replays |
| Broader workflow regression | 315 tests pass across 54 files, excluding benchmark suites |
| Rust engine library | 630 pass, one existing ignored |
| Optimized Rust shared fixtures | Both harnesses pass, covering all 65 cases |
| Optimized Rust shared and field/bundle reporting graphs | Seven pass |
| Elixir test-environment compilation with warnings as errors | Pass |
| Rust engine all-target Clippy with warnings denied | Pass |
| Changed Elixir/Rust formatting and whitespace | Pass |
| Operator-profile, tensor, organization and documentation gates | Pass; 54 operator profiles validated |

Counts overlap and are not coverage percentages. The disposable API databases
were removed after both test processes completed; the development database was
not modified for the tests. The tensor claim remains bounded `verified`
contract/numerical/replay evidence, not remote operational evidence. Overall
qualification stays blocked with zero structural gaps, four maturity gaps,
16 evidence-grade gaps and 11 P0 gaps. Source/document limits remain 800/2,000
lines with zero organization debt.

## Remaining Limits

- The Elixir graph runner still fails fast on operator errors. Rust's opt-in
  branch-local `on_error: skip` recovery is not claimed by this profile. The
  [follow-up branch-recovery report](workflow-branch-recovery-20260925.md) records
  the separate implementation and bounded recovery tests.
  Aligning recovery policy and failure receipts remains a separate task.
- Dedicated domain extractors, guards, quality scorers, generic summary reducers,
  report provenance and arbitrary external plugins need independent parity
  audits. Do not infer their reliability from these four operators.
- Sorting/checked reductions do not provide compensated-sum accuracy, interval
  proofs, full floating-range equivalence, or measured performance improvements.
  Hotspot ID lists remain unbounded by the 32-record sample cap.
- No GUI, installed shell, remote deployment, crash/restart or large-mesh test.
- No version bump, commit, push, packaging or installation in this round.
