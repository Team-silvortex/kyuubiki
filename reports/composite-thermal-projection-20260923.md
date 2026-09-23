# Composite Thermal Projection: 2026-09-23

## Scope And Reproductions

Base revision: `65551b26`, with the preceding feedback-convergence fixes still
in the working tree. This change repairs the next Rust study-level handoff:
heat results to structural temperatures and regional expansion coefficients.
No GUI, runtime dependency, schema version or constitutive law was added.

The initial debug run had 11 failing regressions and three passing controls.
It exposed raw-index rejection of a correctly reordered mesh, reuse of one
source node for repeated target IDs, unchecked result/input indices, ambiguous
element IDs, empty fields producing infinite evidence, and finite temperatures
whose difference overflowed. Small expansion coefficients also inherited an
absolute-difference shortcut instead of a scale-independent relative change.

An actual downstream solve additionally showed that negative coefficients
accepted by the SDK were rejected by the thermal-plane solver. Final tests
therefore enforce that existing solver contract rather than treating a signed
coefficient arithmetic check as evidence for a new material capability.
Two older synthetic unit fixtures now retain complete nodal input metadata
instead of referring to nodes in an empty input mesh.

## Transfer Contract

Both projection APIs share one indexed map. The heat result, retained heat
input and structural target must contain the same nonempty node count. Result
IDs and indices are unique; each index refers to the corresponding retained
input ID and coordinates. Target lookup consumes each source ID exactly once,
rejecting missing/repeated IDs. Temperatures and coordinates must be finite;
coordinate comparisons use the existing absolute `1e-12 m` tolerance.

Expansion matches each selected element by unique ID and the cyclic sequence
of physical node identities, not array positions. Cyclic rotation and reversed
traversal identify the same boundary; repeated/crossed/unknown connectivity is
rejected. The downstream solver still applies its own positive-Jacobian and
material validation. Element and result-array order do not change the transfer.

Finite temperatures must produce finite temperature differences. Expansion
uses the stable four-node mean and the existing linear temperature scaling.
Relative coefficient changes reuse the non-negative scale-independent feedback
helper. Exact zero reference coefficients or zero scaling remain valid;
nonzero products that underflow to zero fail explicitly. Unsupported negative
reference/adjusted coefficients also fail before structural dispatch. Input
models remain unchanged on rejection, allowing a corrected request to rerun.

This is a same-mesh transfer, not remeshing or interpolation.
Negative thermal expansion remains outside the existing plane solver's supported domain.

## Numerical Validation

The 18 non-benchmark regressions pass in debug and optimized release builds.
Tests use real heat and thermal-plane solves where applicable:

- Restrained uniform expansion matches `sigma_x=sigma_y=-E*alpha*delta_T/(1-nu)`
  under reference offsets `0` and `+/-2^40 C`; stress tolerance is `1e-10` relative.
- Reordered node arrays and cyclic connectivity preserve free expansion
  `u=alpha*delta_T*x`, `v=alpha*delta_T*y`, with absolute displacement tolerance
  `1e-14 m` and residual stress below `1e-7 Pa`.
- Nonuniform temperatures `[35,40,45,50] C` retain their node IDs and a
  `7.5 C` mean rise after both transfers and the restrained structural solve.
- Two regions sharing nodes retain independent expansion coefficients even
  when target elements are reordered. One coefficient increases by 1%, the
  other decreases by 2%, and both stresses match the restrained closed form.
- Boundary tests cover identities, input/result correspondence, connectivity,
  incomplete/nonfinite fields, overflow, underflow, exact-zero behavior and
  rejection followed by clean replay.

All 270 SDK library tests and 39 material-research entry tests also pass.
The native `thermal-plane-patch` profile executes all nine commands successfully,
including heat/thermal review, Q4 interpolation, refinement, invalid-input,
reference-shift and these projection tests. The microbenchmark is ignored by
the ordinary profile and was explicitly executed in the release run above.
Overlapping runs are not unique coverage percentages.

Strict Clippy passes for the SDK library/tests and the CLI integration target,
with warnings denied and no lint exemptions. Formatting, `git diff --check`,
validation-profile structure, tensor structure, project organization and
documentation inventory pass. Source/document limits remain 800/2,000 lines.
Global daji readiness remains blocked: four maturity, 16 evidence-grade and
11 P0 gaps are not promoted by this bounded evidence.

## Local Mapping Microbenchmark

macOS ARM64, Apple M2, optimized release build. Each size receives one warmup
and five measurements; the table reports the median. The benchmark reverses
target node order and times only projection, including validation, indexing
and cloning the output. Fixture construction, solving, result assertions and
output destruction are outside the timed region. Synthetic nodal fields do
not constitute connected large-mesh physics solves.

| Nodes | Before (ms) | After (ms) |
| --- | --- | --- |
| 1,000 | 2.155333 | 0.138791 |
| 4,000 | 13.021458 | 0.758958 |
| 16,000 | 169.096042 | 2.108709 |

The 16,000-node sample is about 80 times faster in this local measurement.
The repeated `O(N^2)` search becomes expected `O(N)` indexing/matching, at the
cost of temporary `O(N)` index storage. No persistent cache was introduced.
No end-to-end solver speedup is claimed.

## Reproduction And Limits

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test composite_thermal_projection
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --release --test composite_thermal_projection -- --include-ignored --nocapture --test-threads=1
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-headless-sdk --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-material-explore
./scripts/kyuubiki check-operator-validation --execute --profile thermal-plane-patch --out tmp/composite-thermal-projection-20260923.json
```

This is bounded local transfer and numerical evidence, not general material
qualification, nonlinear constitutive validation, mesh-independent error
estimation, or proof of complete multiphysics stability. No remote, installed
app, Windows, million-node or memory-peak measurement is claimed. No old results
were bulk recomputed, backed up or migrated; invalid disposable artifacts can
be discarded. Existing work and uncommitted feedback fixes are preserved.
