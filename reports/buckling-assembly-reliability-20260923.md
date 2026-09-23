# Buckling assembly reliability, 2026-09-23

This is bounded local buckling-assembly validation, not general structural stability or nonlinear collapse qualification.

Base revision: `b7ca95a2` (`daji 3.3.5`) plus this working tree. The preceding
thermal, modal, transient and harmonic fixes were already committed when this
round began. No version bump, app reinstall or remote deployment is implied.

## Reproduced failures

Two public-solver regressions failed before modification:

- An eight-element pinned Euler beam had first load factor
  `14.621758258307848`. Reversing only element 2 changed it to
  `13.655440364603272`, about 6.6 percent lower, without changing geometry,
  stiffness or reference compression. Assembly used absolute length but
  retained the input edge's node order in its rotation/displacement coupling.
- Scaling a frame's Young's moduli and applied loads by `1e-20` should preserve
  displacement and buckling factors. Instead the frame returned
  `reference load produces no compressive member force`. A fixed `1e-12`
  absolute force threshold disagreed with the positive compression already
  assembled into geometric stiffness.

## Corrections

Beam assembly maps each element from its lower-x endpoint to its higher-x
endpoint. Both elastic and geometric matrices therefore use the same global
positive-x rotation convention. The request's node/element ordering and IDs
are preserved; input data is not rewritten. Compression remains a positive
reference-force magnitude, independent of endpoint order.

Simply removing the absolute cutoff made an existing portal regression fail:
the unloaded crossbeam became active from numerical axial noise. The final
rule suppresses compression at or below `64 * EPSILON * (EA/L) * u_scale`,
where `u_scale` is the largest endpoint translation norm for that member.
This is a local floating-point recovery guard, not an error estimator for the
static solve. The filtered force feeds both activity and geometric stiffness;
the original signed axial force remains in diagnostics. No global maximum
load can suppress an independently supported, weakly compressed member.

The average opposing end force is formed by halving before subtraction, and
nonfinite preload, resolution or local stiffness is an indexed error.
Local frame length uses a stable two-component norm. This does not change the
compression-only approximation into a full tension-stabilized geometric model.

Both operators use shared checked mode expansion in `buckling_math` rather
than duplicating normalization. Load factors must be positive and finite;
residual diagnostics must be finite and nonnegative. The vector must be finite,
nonzero and dimensionally consistent. Normalization first divides by its
largest component, avoiding squaring huge values or losing tiny vectors, then
restores constrained zero DOFs. Residual diagnostics retain the eigensolver's
existing reduced-space normalization, not the returned shape's normalization.

Assembly and result recovery use the existing cooperative cancellation
mechanism. Numerical cancellation remains available through the shared
linear algebra and Jacobi checkpoints. No new task, engine, SDK or result
protocol is introduced, and no physical implementation moved into the engine.
Initial validation and some numerical preparation loops remain synchronous;
no strict cancellation-latency bound is claimed.

## Coverage

- Twelve new tests in `buckling_assembly_reliability` cover independent edge reversal,
  heterogeneous-member node/element reorder, coordinate reflection with
  rotation-sign covariance, an oblique reversed-member cantilever, common
  stiffness/load scaling and reference-load-only scaling. Independent members
  carrying forces `100000` and `1e-15` both remain active in the same model.
- A new portal regression checks inactive crossbeam noise and active column
  compression at three rigid rotations and three common stiffness/load scales
  (`1e-20`, `1`, `1e20`), retaining all requested buckling factors.
- A 280-element mixed-orientation pinned beam crosses the 512-free-DOF dense
  threshold and matches the analytical Euler critical load within `1e-6`
  relative error. Retained 400-element beam/frame Euler checks also run. These
  are bounded sparse regressions, not new million-node benchmarks.
- Negative controls retain rejection of zero/tensile preloads, a free rigid
  translation, an unconnected free node and overflowing element stiffness.
  Fresh valid calls succeed after failure.
- Both operators are cancelled inside assembly, Jacobi iteration, shape
  recovery and final mode collection. Fresh replays match an uninterrupted
  baseline; cancelled calls cannot return modes as partial success.
- Two private kernel tests check recovered shape scales `1e-320`, `1`, `1e308`
  and reject invalid load factors, residuals, vectors, dimensions and mapping.
- Four CLI integration tests build actual Rust headless execution plans and
  resolve engine routes for reversed beams, small scaled frames, indexed
  assembly errors and cancellation/replay. These are in-process tests, not
  remote Agent transport or installed-app tests.

The frame stability assembly is shared with P-Delta and continuation, so the
verification also includes retained adjacent solver and engine workflow tests.
Those regressions do not extend their physical qualification scope.

## Limits

This round does not modify generalized eigensolver filtering, convergence
criteria or conditioning policy. A finite residual diagnostic is not by itself
proof of accuracy. There is no new global lowest-mode guarantee, arbitrary
coefficient-scale certification, material provenance or nonlinear collapse
validation. Very small recovered axial forces can still be affected by static
solution roundoff; replacing a fixed unit-dependent cutoff is not a preload
uncertainty estimator. The full static-frame output range is not audited here.

## Reproduction

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-solver -p kyuubiki-engine -p kyuubiki-headless-sdk --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test buckling_assembly_reliability --test buckling_frame_2d_portal -p kyuubiki-cli --test buckling_assembly_operator
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib buckling_math::tests::recovered_
cargo test --release --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test buckling_assembly_reliability --test buckling_frame_2d_portal -p kyuubiki-cli --test buckling_assembly_operator
./scripts/kyuubiki check-operator-validation --execute --profile buckling-assembly-local-reliability --out tmp/buckling-assembly-20260923.json
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test buckling_beam_1d_input_reliability --test buckling_frame_2d_input_reliability --test buckling_beam_frame_crosscheck --test buckling_beam_1d_clustered_mode --test buckling_frame_2d_clustered_modes --test buckling_frame_2d_portal --test frame_2d_p_delta_closed_form --test frame_2d_material_p_delta_closed_form --test frame_2d_p_delta_explicit_imperfection --test frame_2d_branch_continuation -p kyuubiki-engine --test buckling_beam_workflow --test buckling_frame_workflow --test frame_2d_p_delta_workflow
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --all-targets --no-deps -- -D warnings
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test buckling_assembly_operator --no-deps -- -D warnings
make check-operator-validation check-module-function-coverage-tensor audit-project-organization check-doc-inventory
```

## Verification results

Local macOS ARM64 results:

- Core protocol, solver, engine and headless-SDK library tests: 1138 passed,
  7 existing ignored, no failures.
- All 19 new tests passed in debug and release builds: 2 private mode-recovery
  tests, 12 assembly integration tests, 1 portal scale/rotation regression and
  4 headless engine-route tests. The portal target also retains 2 existing tests.
- The scoped `buckling-assembly-local-reliability` profile executed all 5
  commands successfully, totaling 36 test executions including retained controls.
- Adjacent buckling, P-Delta, material and branch-continuation solver/engine
  regressions: 49 passed across 13 test targets.
- Strict solver all-target and CLI integration-test Clippy checks, touched-file
  formatting and whitespace checks passed.
- Profile registry validation passed with 38 profiles. This is registry
  validation, not execution of all 38 profiles in this round.
- Tensor structure/command checks, project organization audit and documentation
  inventory passed. Source/document limits remain 800/2000 with zero tracked
  line-limit debt. The shared buckling math module is 467 lines; the new assembly
  integration target is 381 lines.

The new tensor claim is local `verified` evidence for numerical validation and
recovery only. Global tensor status remains blocked: 4 maturity gaps,
16 evidence-grade gaps and 11 P0 gaps. No operator-family promotion or closure
of unrelated coverage gaps is implied. No app reinstall, remote deployment,
large benchmark, version bump or Git commit was performed.
