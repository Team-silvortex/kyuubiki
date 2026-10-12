# Electromagnetic diagnostic integrity and fresh-run recovery

Date: 2026-10-12. Current line: daji 3.5.0. Source macOS working-tree overlay
at `69da83e9`. No version bump, commit, installed data migration or app rebuild.

## Reproduced defects

The legacy Elixir electrostatic and magnetostatic reducers filtered malformed
rows and missing scalar/vector samples. They could publish metadata-only success,
erase explicit nonconvergence, select peaks from incomplete evidence, or replace
invalid vector magnitudes with healthy components. Squared norms could overflow
or underflow even when the final magnitude was representable. The initial
seed-0 regression reproduced metadata-only success, ignored nonconvergence and
partial scalar reduction before implementation changed. A fourth failure
identified coarse atom diagnostics rather than the required contextual path.

There was also a native/service configuration discrepancy: Elixir supported
`charge_density_source=elements`, but native electrostatic diagnostics always
read nodes. An unknown selector also silently chose nodes in the old service.
These are result-integrity and contract defects, not demonstrated security exploits.

## Changes

`WorkflowDiagnosticSamples` replaces the duplicate permissive service helpers
with shared checked scalar distributions and vector/scalar peaks. It follows the
native sample-admission rules without implementing another physical solver.

- Entirely absent optional metric groups stay absent. Once a group has any
  samples, every row must provide it; configured mappings are always required.
- Every consumed row must be an object. Explicit invalid values and earlier
  aliases reject with the original collection, index and field path.
- Metadata alone is not evidence. The extraction must measure at least one group,
  and cannot erase explicit nonconvergence or malformed convergence markers.
- Scalar-only and magnitude-only results remain supported. Present components
  are checked even beside an explicit magnitude. Requested third components
  must exist and contribute to derived norms; invalid magnitudes cannot fall back.
- Stable norms preserve representable very large/small vectors. Required sums
  and spans reject overflow instead of raising or publishing a null result.
- Explicit source/field/prefix settings are validated, rather than replaced by
  defaults. False configuration is no longer converted to an empty map by the
  electromagnetic operator dispatcher; absent/null configuration remains valid.
- Mixed declared aliases participate in complete reductions. Magnetic stored
  energy remains a whole-group legacy fallback, never a patch for partial density
  samples. Ordinary equal peaks select the last row, as in the native reducer.

Both native Rust and Elixir now honor `node`/`nodes` and `element`/`elements`
charge-density selection, including custom collection names. Missing selected
fields and unknown selectors reject, with no fallback to another collection.
The Rust change reuses the checked distribution accumulator and cancellation
checkpoints rather than adding an independent reducer.

Existing successful metric keys remain available, with native-style distribution
counts, sums, spans and peak aliases additionally exposed by the service.
Potential distributions now check their emitted sum as well as mean and span.
Accumulator state is bounded per group; filtered/sorted copies of all samples
are no longer retained by these service reductions. This is not an RSS benchmark.

The old magnitude-only positive fixture explicitly requested nonexistent X/Y
fields while expecting success. It now omits those optional mappings; separate
negative controls require explicit missing mappings to fail. The graph fixture
uses the documented `fail`/`skip` policies, not an invented `stop` value.

## Verification

The new service matrix has 34 tests across both domains, including primary and
secondary scalars, aliases, vector components, source configuration, first/middle/
last missing samples, a 4,096-row corrupt tail, independent scalar references,
zero values and representable/unrepresentable numeric extremes.

Four graph tests execute 32 controlled runs: 16 rejected inputs and 16 fresh
corrected reruns through input, diagnostics, guard, JSON export and output.
Under `fail`, the graph stops at the diagnostic with its reason. Under `skip`,
the diagnostic remains failed and dependent nodes cannot complete or publish
artifacts. Both policies prove the decision/export callbacks were not invoked
on bad evidence, and complete reports are produced after explicit correction.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 599985
```

Result: 1,627 total, 1,556 passed, 71 conditional skips, zero failures. This
includes the 38 new service tests and existing normal-result controls. Skipped
environments are not execution evidence. The focused new selection passes 38.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib diagnostic --quiet
cargo test --locked --offline -p kyuubiki-engine --lib workflow_metric_integrity --quiet
```

Results: 69 diagnostic-filter tests and 22 metric-integrity-filter tests pass.
These selections overlap and must not be added as distinct coverage. The native
diagnostic selection includes the new charge-source regression; its dedicated
integrity file now contains 27 passing tests.

## Limits and next work

Service graph runs use controlled raw fields and direct registered Elixir
callbacks. They are not new API jobs, live Rust Agent dispatches, numerical
solver qualification, remote/installed tests, or automatic durable resumption.
Native Engine tests likewise exercise controlled diagnostic inputs. No blanket
cross-language equivalence, scale or performance qualification is claimed.

Legacy thermal/thermo raw reducers and service quality scoring still need audit.
A downstream guard cannot detect a plausible number fabricated upstream, and
repairing diagnostic admission alone cannot qualify research results. The tensor
receives scoped control-plane contract/execution/recovery evidence and separate
Engine contract/execution evidence, not new Engine recovery proof. Global maturity
and release qualification are unchanged.

Subsequent work: the [thermal diagnostic follow-up](thermal-diagnostic-integrity-20261012.md)
replaces the permissive thermal/thermo raw reducers and aligns native displacement
configuration aliases. Service quality scoring and broader thermal decision
admission remain separate work; the historical counts above are unchanged.
