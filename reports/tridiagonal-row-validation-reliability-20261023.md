# Tridiagonal Row Validation Reliability

Date: 2026-10-23

Version line: daji 3.6.x, following committed `9aed9a2f` (`daji 3.6.8`).
No version bump, application package, commit or deployment is included.

Ordinary and equilibrated tridiagonal factors now use the same original-equation
admission gate. A large equation cannot relax another row's error scale.
Subnormal products are checked with binary normalization rather than silently
disappearing. Legitimate rounded tails remain usable within a row-local bound.

## Reproduced failures

The following controls failed before the repair:

- For `A = diag(1, 1)`, `b = [1e20, 1]` and candidate `x = [1e20, 2]`,
  the old ordinary-path global floor reported about `2.25e-15`, hiding a
  componentwise error of `1/3` in the second equation.
- For the smallest positive subnormal `q`, the product `0.5 * q` vanished
  before validation. Conversely, two such products whose sum is `q` were
  reported as a full relative error instead of a satisfied equation.
- Solving `diag(1, 4) x = [1, q]` returned `[1, 0]`. A nonzero weak load
  with no representable response was certified beside an ordinary strong row.
- Deliberately changing a prepared factor's saved weak equation while leaving
  its numerical factors unchanged was hidden by an unrelated large load.

Both signs, finite large loads, subnormal operands and row scaling are covered.
The changed completion observation is a new one-pass cancellation contract,
not an additional pre-existing numerical failure.

## Implementation and compatibility boundary

`chain_tridiagonal_residual.rs` owns the shared gate. Each row uses four fixed
terms: RHS and up to three coefficient/response products. Ordinary arithmetic
keeps its accumulation order. Exceptional products, sums or residuals are
normalized using the existing `linear_binary_scale` primitives. Non-finite
operands, mismatched dimensions and invalid tolerance are rejected explicitly.

The old ordinary-path `global_scale * EPSILON / tolerance` floor is removed.
There is no whole-system retry or node-count-sized residual buffer. Validation
uses O(1) auxiliary storage and O(n) row visits; the overall factor and solution
storage remain O(n). This is a structural memory improvement, not a measured
throughput or peak-RSS benchmark.

A strict row-only error initially rejected the existing 10,000-node transient
spring-chain test: its far-tail responses reach subnormal and zero quantization.
The repair therefore includes a local representation allowance, not a return
to global scaling. For subnormal or zero unknowns, the row allowance is
`sum(abs(a_ij) * q / 2)`, evaluated in the same normalized domain when needed.
The metric is `max(abs(b - A*x) - allowance, 0) / row_scale`. The allowance
is never used when a nonzero RHS has all row products erased.

The negative controls still reject an entirely erased load and errors larger
than the local quantum allowance. An additional 1,026-row `4/-1/-1` chain
checks its first 80 near-load responses against the decaying Green's function
`r^(distance+1)`, where `r = 1/(2+sqrt(3))`; the remote-boundary correction is
below the floating-point range at this length. It also requires both subnormal
and zero tail values and forbids dense fallback.

Factorization, substitution, pivot thresholds, tolerance constants and dense
fallback limits are unchanged. Engine/operator separation is unchanged.
`SparseResidual` observes entry, every 256 rows and completion at n, rather
than the previous ordinary-path 2n completion. Cancellation and failure leave
the immutable factor reusable. These are cooperative interruption points,
not persisted power-loss recovery checkpoints.

## Added regression coverage

Thirteen tests were added: eleven solver unit tests, one solver integration
test and one engine workflow test. They cover:

- Row-local negative controls, scaling invariance and half-quantum products.
- Normal, pivoted and equilibrated paths followed by clean factor replay.
- Entry, intermediate and final cancellation without publishing a result.
- Invalid dimensions, NaN/infinite operands and invalid tolerances.
- Valid long-chain quantized tails and multi-quantum negative controls.
- Borrowed/owned spring APIs, reversed numbering/orientation and both signs.
- Direct dispatch, JSON round trip, workflow rejection and successful replay
  after changing the weak load from `q` to the representable `4q`.

The equilibrated negative control rejects at the earlier RHS range-loss gate;
it is not claimed as a residual-stage rejection. Replay at `4q` succeeds.

## Validation

Completed with Rust 1.88.0, offline locked dependencies and existing caches:

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver library excluding `modal_frame_spectrum::` | 619 passed, 9 ignored | 619 passed, 9 ignored |
| Complete engine library unit suite | 711 passed, 1 ignored | 711 passed, 1 ignored |
| 42 solver/engine/CLI/agent integration targets | 247 passed | 247 passed |
| Total excluding repeated focused runs | 1,577 passed, 10 ignored | 1,577 passed, 10 ignored |

The new tests are included above, not counted a second time. Separate focused
runs of `chain_tridiagonal` (39 passed), `linear_` (183 passed, 3 ignored) and
`solver_control` (12 passed) also passed on both platforms; they are subsets of
the 619 passing solver tests. Integration coverage includes the 10,000-node
transient chain and 52 live-agent cancellation tests.

An additional unfiltered solver-library attempt on each platform was explicitly
stopped with SIGINT after entering the long-running modal matrix. Those partial
runs are not counted as passing. The completed bounded solver run filters out
292 `modal_frame_spectrum::` tests; full modal requalification remains outside
this report. Existing opt-in benchmarks remain ignored. This is not full-project
coverage, a new large-mesh performance benchmark or a production qualification.

Local Clippy passed for all solver and engine targets with `-D warnings`.
Workspace formatting and whitespace checks passed. Changed source/config files
remain below 800 lines and this report below 2,000 lines. No additional build
root, retained input dump or package archive was created.

Native operator validation passed with 70 profiles and `executed=false`.
Tensor structure and command checks passed: 13 modules, 11 paradigms,
0 structural gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.
Overall readiness remains blocked. These metadata checks are separate from
the executed numerical tests above; no general maturity or release claim is
promoted by this component evidence.

## Reproduction

Run from the repository root with the pinned Rust toolchain and dependencies:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver --lib -- --skip modal_frame_spectrum:: --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-engine --lib -- --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-cli \
  --test spring_mixed_scale_equilibrium \
  --test spring_1d_closed_form --test spring_1d_refinement \
  --test spring_vector_closed_form --test spring_vector_refinement \
  --test dynamic_spring_closed_form --test transient_spring_reliability \
  --test transient_spring_balance --test harmonic_spring_reliability \
  --test nonlinear_spring_globalization --test nonlinear_spring_recovery \
  --test nonlinear_spring_scaling --test nonlinear_spring_tolerance \
  --test nonlinear_spring_range_reliability --test nonlinear_spring_topology \
  --test nonlinear_spring_tangent_pivot --test nonlinear_spring_softening_arithmetic \
  --test nonlinear_spring_equation_scaling --test heat_bar_closed_form \
  --test transient_heat_bar_closed_form --test transient_heat_bar_reliability \
  --test thermal_bar_1d_closed_form --test bar_1d_closed_form \
  --test bar_1d_input_reliability --test torsion_input_reliability \
  --test acoustic_bar_closed_form --test acoustic_bar_refinement \
  --test acoustic_bar_reliability --test magnetostatic_bar_closed_form \
  --test magnetostatic_bar_reliability --test advection_diffusion_bar_closed_form \
  --test advection_diffusion_bar_reliability --test advection_diffusion_bar_upwind \
  --test spring_mixed_scale_workflow --test nonlinear_spring_workflow \
  --test nonlinear_spring_topology_workflow --test nonlinear_spring_recovery_workflow \
  --test result_admission_workflow --test harmonic_spring_operator \
  --test transient_spring_operator --test nonlinear_spring_range_operator \
  --test agent_solver_cancel_live --no-fail-fast -- --quiet
cargo clippy --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --all-targets -- -D warnings
cargo fmt --manifest-path workers/rust/Cargo.toml --all -- --check
```

This is bounded tridiagonal admission evidence, not general conditioning, physical qualification or durable restart.
