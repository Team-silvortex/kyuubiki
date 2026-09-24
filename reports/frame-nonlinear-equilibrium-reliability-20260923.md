# Frame nonlinear-equilibrium reliability, 2026-09-23

This is bounded local support-load isolation validation, not general nonlinear equilibrium or mixed-scale accuracy qualification.

Base revision: `a7557be7` (`daji 3.3.6`) plus the preceding working-tree
initial-stress preflight correction. Existing edits are preserved. There is
no version bump, Git commit, deployment or app rebuild in this round.

## Reproduced Defects

Four public-solver controls were added before the correction. Three failed;
the existing linearized free-equation control passed:

- A large load applied directly at a constrained support made the
  corotational path report convergence before computing the ordinary
  cantilever's nonzero displacement.
- The same physically irrelevant-to-free-motion support loads changed
  arc-length equilibrium and step adaptation.
- A one-iteration, zero-cutback path that correctly failed without those
  support loads was incorrectly returned as converged with them.

All input values in these controls are finite. Large magnitudes test
normalization isolation; they are not material data or engineering loading
recommendations. Support forces and moments are valid reaction inputs, not
additional applied loads on the free equations.

## Correction

The shared nonlinear residual norm takes the reduced free-DOF map explicitly.
It computes the reference infinity norm only over those DOFs, while retaining
the existing `max(reference_norm, 1)` and `max(abs(load_factor), 1)` divisors.
Sequential division still avoids multiplying two large scales. Malformed
map dimensions/indices and nonfinite values produce an infinite invalid
metric rather than a false zero or indexing panic.

All production callers pass their actual reduction map: load-control
convergence and backtracking, arc-length correction, parameter-continuation
correction, and branch-modal correction/backtracking. No reduced reference
vector allocation was added solely for this metric.

The linearized path keeps its existing row-local equilibrium verification.
No public schema, solver tolerance, cutback budget, material law or commit
policy changes. Failed nonlinear trials remain nonconverged and retain the
last accepted state rather than committing material history under a false
success. Engine dispatch and the headless task format remain decoupled from
these solver internals.

## Tests And Reproduction

Thirteen new regressions cover six kernel cases, four public-solver paths
and three Rust headless-plan cases. They check constrained-load exclusion,
non-prefix/permuted free maps, invalid dimensions and indices, nonfinite
values, safe large-scale normalization, load-control and arc-length paths,
failed-iteration classification, cyclic history, failed-trial commit safety
and fresh replay. Retained branch and continuation tests exercise adjacent
routes; they are not new constrained-load negative controls for every branch
selection strategy.

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib equilibrium_tests
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test frame_2d_equilibrium_reliability -p kyuubiki-cli --test frame_2d_material_operator --test frame_2d_p_delta_operator
./scripts/kyuubiki check-operator-validation --execute --profile frame-nonlinear-equilibrium-local-reliability --out tmp/frame-nonlinear-equilibrium-20260923.json
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test frame_2d_material_operator --test frame_2d_p_delta_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

## Verification

Local macOS ARM64 results on this working tree:

- Core protocol, solver, engine and headless-SDK libraries: 1175 passed,
  7 existing ignored, no failures.
- All thirteen new cases passed in debug and release. Release groups passed
  25 tests in total: 6 kernel, 4 public-solver and 15 headless-route cases,
  including 12 retained controls.
- Expanded mechanical regression: 99 passed across 16 public-solver,
  headless and workflow targets. Initial-stress, composite, cyclic material,
  adaptive fiber, linearized, arc-length and branch controls remain passing.
- The `frame-nonlinear-equilibrium-local-reliability` profile executed all
  four commands successfully, totaling 69 test executions. This is not an
  execution of the entire validation registry.
- Strict solver all-target and CLI integration-test Clippy checks, touched
  source formatting and whitespace checks passed.
- Registry validation passed with 43 profiles. Tensor structure/command
  checks, documentation inventory and the 800/2000 source/document limits
  passed with zero tracked line-limit debt.

The global tensor still reports 4 maturity gaps, 16 evidence-grade gaps and
11 P0 gaps; Daji qualification remains blocked. Generated profile results
remain ignored under `tmp/`, not added as build or data artifacts.

## Limits

This corrects support-load pollution of an existing free-equation norm, not
every possible nonlinear false-convergence condition. The global free norm
still combines force and moment values and retains its historical absolute
and load-factor floors. Per-equation scaling, length-unit normalization and
arbitrary reference-load/load-factor reparameterization need separate
validation and design, particularly around zero-load plastic unloading.

No remote Agent, installed GUI, throughput, large-mesh benchmark, general
structural qualification or durable restart is claimed. Global Daji
qualification gaps are not closed by this bounded local evidence.
