# Thermal diagnostic integrity and fresh-run recovery

Date: 2026-10-12. Current line: daji 3.5.0. Source macOS working-tree overlay
at `69da83e9`. No version bump, commit, app rebuild or installed data migration.

## Reproduced defects

Legacy service thermal/thermo extraction silently filtered missing/corrupt
samples and could return metadata-only success. A present invalid magnitude
could fall back to valid components. The seed-0 baseline stopped after three
reproductions of these defects before implementation changed. Existing helpers
also ignored explicit nonconvergence, squared vector components before taking
their norm, and selected the first available stress/strain component axis rather
than comparing all selected axes. These are result-integrity defects, not
demonstrated security exploits or new physical-solver failures.

A separate native regression reproduced rejection of a valid custom displacement
vector using legacy `ux_field`/`uy_field`/`uz_field` configuration: the Engine
reported no diagnostic fields because it ignored these service-supported names.

## Changes

Thermal and thermo service extraction now use `WorkflowDiagnosticSamples`, with
domain field selection isolated in `WorkflowThermalDiagnostics`. The old duplicate
thermal statistics module and permissive reduction helpers are removed. Guards
and pair benchmarks remain separate from raw reduction and physical solving.

- Empty metadata and malformed or false convergence markers cannot become
  successful measurement evidence. Every consumed row must be an object.
- Optional metric groups may be wholly absent, but partial groups reject rather
  than shrinking counts. Explicit configured fields are always required.
- Temperature, temperature delta, heat loads, gradients, flux, displacement,
  stress and strain retain original collection/index/field paths on rejection.
  Healthy temperature or flux cannot hide damaged secondary groups.
- Present invalid aliases or magnitudes cannot fall back to another value.
  Stable vector norms preserve representable large/small values; required
  accumulated sums and spans reject overflow rather than publishing nulls.
- Scalar-only and magnitude-only evidence remains supported. Present vector
  components are still checked; explicitly selected third components must exist.
- Mixed declared displacement and von Mises aliases are resolved per row, not by
  choosing one incomplete alias group. Stress can fall back to the root summary
  or components only when the preceding optional group is wholly absent.
- Stress/strain component fallbacks compare all present axes by absolute value
  and preserve the winning sign. Explicit scalar strain uses the native signed
  maximum, not the old service absolute maximum. Regression controls distinguish
  these semantics; a scalar peak must not be interpreted as an absolute envelope.
- Heat-load cancellation retains nonzero loaded-node counts. Temperature extrema
  IDs, heat-load peak IDs and existing service metric aliases remain available;
  absent groups do not gain fabricated zero metrics. Missing scalar IDs follow
  the native thermo `unknown` alias while explicit null IDs remain null.
- Primary displacement mappings win by presence, including invalid values.
  Native and service extraction both support legacy mapping names without
  converting an invalid primary setting into an apparently successful fallback.
  The operator dispatcher likewise no longer converts false config to defaults.

Service accumulators retain bounded state rather than filtered/sorted sample
copies. This is an implementation property, not an RSS or throughput benchmark.
Service-only extrema/heat-load identity fields remain additional aliases; this
round does not claim complete cross-language floating-point/tie equivalence.

## Verification

The new service selection contains 27 admission/reduction tests plus four graph
tests. It includes first/middle/last corrupt rows, a 4,096-row corrupt tail,
secondary heat/gradient groups, signed stress/strain controls, numeric extremes,
custom collection/field mapping and primary/legacy precedence.

The graph tests execute 32 controlled runs: 16 rejected inputs and 16 fresh
corrected reruns through input, diagnostics, thermal guard, JSON export and output.
With `fail`, rejection remains attached to the diagnostic node. With `skip`, that
node still fails and dependent nodes cannot publish output. Both policies check
that decision/export callbacks never run on invalid raw evidence. Corrected data
then produces a complete blocking guard report, not a fabricated passing report.

From `apps/web`:

```sh
mix test test/kyuubiki_web/workflow_*test.exs test/kyuubiki_web/api test/kyuubiki_web/orchestra/workflow_*test.exs test/kyuubiki_web/library test/kyuubiki_web/storage --seed 599985
```

Final result: 1,658 total, 1,587 passed, 71 conditional skips, zero failures.
The 31 new service tests are included. Conditional skips are not tested coverage.

From `workers/rust`:

```sh
cargo test --locked --offline -p kyuubiki-engine --lib workflow_thermo_integrity --quiet
cargo test --locked --offline -p kyuubiki-engine --lib diagnostic --quiet
cargo test --locked --offline -p kyuubiki-engine --lib workflow_thermal_catalog_contract --quiet
cargo test --locked --offline -p kyuubiki-engine --lib workflow_metric_integrity --quiet
```

Results: 22 thermo-integrity, 69 diagnostic-filter, five thermal-catalog and 22
metric-integrity tests pass. Filter selections overlap and must not be summed
as distinct coverage. Three native thermo regressions were added, protecting
legacy displacement mappings, invalid primary/legacy precedence and signed
scalar strain semantics. Format and diff checks also pass.

## Limits and next work

The graph tests use controlled raw fields and registered service callbacks.
They are not new API jobs, live Agent dispatches, fresh physical solves, remote
or installed acceptance, and not automatic resumption of the same interrupted job.
Native tests exercise controlled extraction inputs as well.
This does not add native recovery evidence or qualify numerical accuracy at any
mesh scale.

Service quality scoring still needs strict configuration/convergence admission.
Thermal guard/pair admission beyond the earlier missing-metric checks also remains
separate work, including malformed labels, weights and numeric arithmetic. These
gaps are not covered by repairing the upstream raw reducers alone.

The tensor records service contract/execution/fresh-run recovery and separate
Engine field-mapping contract/execution evidence. Global maturity, release
qualification and the historical evidence counts remain unchanged.

Subsequent work: the [quality and thermal decision follow-up](quality-and-thermal-decision-integrity-20261012.md)
closes the service domain-score admission gaps and adopts shared thermal guard/
pair validation. The historical counts above are unchanged; independent
composite-objective and candidate-ranking admission still require review.
